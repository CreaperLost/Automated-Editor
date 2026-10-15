//! The private channel between the running app and `aeroedits --mcp-stdio`.
//!
//! On Windows it is a named pipe whose access list names only this Windows account, that
//! refuses connections from other computers, and that the app must be first to create (so
//! nothing can sit on its name). Elsewhere it is a Unix socket in a folder only this account
//! can enter. Either way no network port is open, and the bridge checks that the other end
//! runs as the same account before it passes anything on.
use tokio::io::{AsyncRead, AsyncWrite};

/// Where the app listens: a pipe name on Windows, a socket path elsewhere.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint(pub String);

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub trait Stream: AsyncRead + AsyncWrite + Send + Unpin + 'static {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin + 'static> Stream for T {}

pub use imp::{connect, default_endpoint, Listener};

/// Debug builds only: `AEROEDITS_MCP_DEV=<name>` gives a development copy (and the bridge
/// agents start from it) an endpoint of its own, so it runs beside the installed app, which
/// already owns the usual one. Release builds always use the usual endpoint.
fn dev_suffix() -> String {
    #[cfg(debug_assertions)]
    if let Ok(name) = std::env::var("AEROEDITS_MCP_DEV") {
        let name: String = name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(16)
            .collect();
        if !name.is_empty() {
            return format!("-{name}");
        }
    }
    String::new()
}

#[cfg(windows)]
mod imp {
    use super::Endpoint;
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    use tokio::net::windows::named_pipe::{
        ClientOptions, NamedPipeClient, NamedPipeServer, PipeMode, ServerOptions,
    };
    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, ERROR_PIPE_BUSY, HANDLE, HLOCAL};
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{
        EqualSid, GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
        TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::System::Pipes::GetNamedPipeServerProcessId;
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    /// A process's user, as the token information that holds its SID.
    struct UserSid {
        /// `TOKEN_USER` followed by the SID it points into; u64s keep it aligned.
        buffer: Vec<u64>,
    }

    impl UserSid {
        fn of(process: HANDLE) -> Result<Self, String> {
            unsafe {
                let mut token: HANDLE = std::ptr::null_mut();
                if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
                    return Err("Could not read the process's account".into());
                }
                let mut len = 0u32;
                GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len);
                let mut buffer = vec![0u64; (len as usize).div_ceil(8).max(1)];
                let ok = GetTokenInformation(
                    token,
                    TokenUser,
                    buffer.as_mut_ptr().cast(),
                    (buffer.len() * 8) as u32,
                    &mut len,
                );
                CloseHandle(token);
                if ok == 0 {
                    return Err("Could not read the process's account".into());
                }
                Ok(Self { buffer })
            }
        }

        fn current() -> Result<Self, String> {
            Self::of(unsafe { GetCurrentProcess() })
        }

        fn sid(&self) -> *mut c_void {
            unsafe { (*(self.buffer.as_ptr() as *const TOKEN_USER)).User.Sid }
        }

        fn text(&self) -> Result<String, String> {
            unsafe {
                let mut text: *mut u16 = std::ptr::null_mut();
                if ConvertSidToStringSidW(self.sid(), &mut text) == 0 {
                    return Err("Could not name this account".into());
                }
                let len = (0..).take_while(|&i| *text.add(i) != 0).count();
                let out = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
                LocalFree(text as HLOCAL);
                Ok(out)
            }
        }

        fn same(&self, other: &UserSid) -> bool {
            unsafe { EqualSid(self.sid(), other.sid()) != 0 }
        }
    }

    /// One pipe per Windows account, so two people on one PC each reach their own app.
    pub fn default_endpoint() -> Result<Endpoint, String> {
        Ok(Endpoint(format!(
            r"\\.\pipe\aeroedits-mcp-{}{}",
            UserSid::current()?.text()?,
            super::dev_suffix()
        )))
    }

    /// Security attributes that let only this account open the pipe: no one else, not even
    /// administrators or the system account, and no inherited entries (`P`).
    struct OwnerOnly {
        descriptor: PSECURITY_DESCRIPTOR,
        attributes: SECURITY_ATTRIBUTES,
    }

    // The descriptor is only read, by the system, while pipes are created.
    unsafe impl Send for OwnerOnly {}
    unsafe impl Sync for OwnerOnly {}

    impl OwnerOnly {
        fn new() -> Result<Box<Self>, String> {
            let sid = UserSid::current()?.text()?;
            let sddl: Vec<u16> = format!("D:P(A;;GA;;;{sid})")
                .encode_utf16()
                .chain([0])
                .collect();
            let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
            let ok = unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                return Err("Could not make the pipe private".into());
            }
            let mut out = Box::new(Self {
                descriptor,
                attributes: SECURITY_ATTRIBUTES {
                    nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                    lpSecurityDescriptor: descriptor,
                    bInheritHandle: 0,
                },
            });
            out.attributes.lpSecurityDescriptor = out.descriptor;
            Ok(out)
        }

        fn pointer(&self) -> *mut c_void {
            &self.attributes as *const SECURITY_ATTRIBUTES as *mut c_void
        }
    }

    impl Drop for OwnerOnly {
        fn drop(&mut self) {
            unsafe { LocalFree(self.descriptor as HLOCAL) };
        }
    }

    fn create(name: &str, security: &OwnerOnly, first: bool) -> std::io::Result<NamedPipeServer> {
        let mut options = ServerOptions::new();
        options
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .pipe_mode(PipeMode::Byte);
        unsafe { options.create_with_security_attributes_raw(name, security.pointer()) }
    }

    pub struct Listener {
        name: String,
        security: Box<OwnerOnly>,
        /// The instance waiting for the next client.
        next: NamedPipeServer,
    }

    impl Listener {
        pub fn bind(endpoint: &Endpoint) -> Result<Self, String> {
            let security = OwnerOnly::new()?;
            let next = create(&endpoint.0, &security, true).map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    "Another AeroEdits window is already ready for agents. Close it first.".into()
                } else {
                    format!("Could not open the agent pipe: {e}")
                }
            })?;
            Ok(Self {
                name: endpoint.0.clone(),
                security,
                next,
            })
        }

        /// Waits for an agent's bridge and hands back its connection.
        pub async fn accept(&mut self) -> std::io::Result<NamedPipeServer> {
            self.next.connect().await?;
            let fresh = create(&self.name, &self.security, false)?;
            Ok(std::mem::replace(&mut self.next, fresh))
        }
    }

    pub async fn connect(endpoint: &Endpoint) -> Result<NamedPipeClient, String> {
        const NOT_RUNNING: &str = super::super::bridge::NOT_RUNNING;
        let client = loop {
            match ClientOptions::new().open(&endpoint.0) {
                Ok(client) => break client,
                // Every instance is taken for the moment: the app makes a new one at once.
                Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(_) => return Err(NOT_RUNNING.into()),
            }
        };
        server_runs_as_me(&client)?;
        Ok(client)
    }

    /// Refuses a pipe that another account made under our name.
    fn server_runs_as_me(client: &NamedPipeClient) -> Result<(), String> {
        const NOT_OURS: &str = "The agent pipe belongs to another account; not connecting.";
        unsafe {
            let mut pid = 0u32;
            if GetNamedPipeServerProcessId(client.as_raw_handle() as HANDLE, &mut pid) == 0 {
                return Err(NOT_OURS.into());
            }
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if process.is_null() {
                return Err(NOT_OURS.into());
            }
            let theirs = UserSid::of(process);
            CloseHandle(process);
            if !theirs?.same(&UserSid::current()?) {
                return Err(NOT_OURS.into());
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
mod imp {
    use super::Endpoint;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::path::Path;
    use tokio::net::{UnixListener, UnixStream};

    /// A socket in a folder of the app's config folder that only this account may enter.
    pub fn default_endpoint() -> Result<Endpoint, String> {
        let dir = crate::transcript::settings::config_dir().join("agents");
        let name = format!("mcp{}.sock", super::dev_suffix());
        Ok(Endpoint(dir.join(name).to_string_lossy().into_owned()))
    }

    fn private_dir(dir: &Path) -> Result<(), String> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|e| e.to_string())?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())
    }

    fn mine(stream: &UnixStream) -> bool {
        stream
            .peer_cred()
            .is_ok_and(|cred| cred.uid() == unsafe { libc::getuid() })
    }

    pub struct Listener {
        listener: UnixListener,
    }

    impl Listener {
        pub fn bind(endpoint: &Endpoint) -> Result<Self, String> {
            let path = Path::new(&endpoint.0);
            if let Some(dir) = path.parent() {
                private_dir(dir)?;
            }
            // A socket left by an app that quit: nothing listens on it any more.
            if std::os::unix::net::UnixStream::connect(path).is_ok() {
                return Err("Another AeroEdits window is already ready for agents.".into());
            }
            let _ = std::fs::remove_file(path);
            let listener = UnixListener::bind(path)
                .map_err(|e| format!("Could not open the agent socket: {e}"))?;
            Ok(Self { listener })
        }

        pub async fn accept(&mut self) -> std::io::Result<UnixStream> {
            loop {
                let (stream, _) = self.listener.accept().await?;
                if mine(&stream) {
                    return Ok(stream);
                }
            }
        }
    }

    pub async fn connect(endpoint: &Endpoint) -> Result<UnixStream, String> {
        let stream = UnixStream::connect(&endpoint.0)
            .await
            .map_err(|_| super::super::bridge::NOT_RUNNING.to_string())?;
        if !mine(&stream) {
            return Err("The agent socket belongs to another account; not connecting.".into());
        }
        Ok(stream)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
    use windows_sys::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertStringSidToSidW,
        GetSecurityInfo, SDDL_REVISION_1, SE_KERNEL_OBJECT,
    };
    use windows_sys::Win32::Security::{
        EqualSid, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
    };

    /// Whether two SIDs in SDDL's text name the same account. SDDL writes some accounts by an
    /// alias rather than as `S-1-5-...`: `LA` for the built-in Administrator, which CI runs as.
    fn same_account(a: &str, b: &str) -> bool {
        let sid = |text: &str| unsafe {
            let wide: Vec<u16> = text.encode_utf16().chain([0]).collect();
            let mut sid: PSID = std::ptr::null_mut();
            assert_ne!(
                ConvertStringSidToSidW(wide.as_ptr(), &mut sid),
                0,
                "not a SID: {text}"
            );
            sid
        };
        let (a, b) = (sid(a), sid(b));
        unsafe {
            let same = EqualSid(a, b) != 0;
            LocalFree(a as HLOCAL);
            LocalFree(b as HLOCAL);
            same
        }
    }

    /// The pipe's access list as Windows holds it, in SDDL.
    fn dacl(handle: std::os::windows::io::RawHandle) -> String {
        unsafe {
            let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
            let status = GetSecurityInfo(
                handle,
                SE_KERNEL_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut descriptor,
            );
            assert_eq!(status, 0, "GetSecurityInfo failed");
            let mut text: *mut u16 = std::ptr::null_mut();
            let ok = ConvertSecurityDescriptorToStringSecurityDescriptorW(
                descriptor,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut text,
                std::ptr::null_mut(),
            );
            assert_ne!(ok, 0);
            let len = (0..).take_while(|&i| *text.add(i) != 0).count();
            let out = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
            LocalFree(text as HLOCAL);
            LocalFree(descriptor as HLOCAL);
            out
        }
    }

    #[test]
    fn only_this_account_may_open_the_pipe_and_only_one_app_may_own_it() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let id = uuid::Uuid::new_v4().simple().to_string();
            let endpoint = Endpoint(format!(r"\\.\pipe\aeroedits-mcp-test-{id}"));
            let mut listener = Listener::bind(&endpoint).unwrap();
            // The listener and the connection it accepted stay open until the end.
            let accepted = tokio::spawn(async move {
                let server = listener.accept().await;
                (listener, server)
            });
            let client = connect(&endpoint).await.unwrap();
            let (_listener, server) = accepted.await.unwrap();
            let _server = server.unwrap();

            let me = imp::default_endpoint().unwrap().0;
            let me = me.rsplit("aeroedits-mcp-").next().unwrap().to_string();
            let sddl = dacl(client.as_raw_handle());
            // Protected (nothing inherited) and a single allow entry: this account.
            assert!(sddl.starts_with("D:P(A;;"), "{sddl}");
            assert_eq!(sddl.matches("(A;").count(), 1, "{sddl}");
            let account = sddl
                .strip_suffix(')')
                .and_then(|s| s.rsplit(";;;").next())
                .unwrap_or_default();
            assert!(same_account(account, &me), "{sddl} does not allow {me}");
            // And the comparison does tell accounts apart.
            assert!(!same_account("LA", "SY"));

            assert!(
                Listener::bind(&endpoint).is_err(),
                "a second owner was allowed"
            );
        });
    }
}
