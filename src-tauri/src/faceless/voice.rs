//! Narration (FCL-4), ported from the explainer's `voice.py` and `film.narrate`. The channel's
//! one voice (Will on Eleven v4 Turbo, through OpenRouter on the user's key) reads each scene
//! of the accepted script. Each scene is its own file, so a changed scene is the only one paid
//! for again; the takes are sped up to the channel's pace without changing pitch (OpenRouter
//! refuses `speed` for this model) and laid end to end with a short gap into one narration
//! file, which goes on the timeline as speech. The app's own transcription then gives the
//! words their times, and model names and numbers are checked against what was heard: a
//! misheard name or number is a wrong fact in the ear.
use super::script::Script;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The explainer's locked voice: "Will, on Eleven v4 Turbo. Do not change it."
pub const MODEL: &str = "elevenlabs/eleven-v4-turbo";
pub const VOICE: &str = "will";
const SPEECH_URL: &str = "https://openrouter.ai/api/v1/audio/speech";
const MODELS_URL: &str = "https://openrouter.ai/api/v1/models?output_modalities=speech";
/// Silence before the first word, so an opening sound doesn't bury it.
pub const LEAD_US: u64 = 250_000;
/// Silence after each scene: the explainer's `SCENE_PADDING`.
pub const PADDING_US: u64 = 450_000;
/// The production's narration files, in `faceless/`.
pub const FOLDER: &str = "voice";
const MANIFEST: &str = "manifest.json";
const MAX_SCENE_CHARS: usize = 5_000;
/// Scenes voiced at once.
const AT_ONCE: usize = 4;

/// The channel's voice, from its profile.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct VoiceSettings {
    /// OpenRouter speech model.
    pub model: String,
    pub voice: String,
    /// The pace the takes are played at (1.2: a fifth faster than read).
    pub speed: f64,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            model: MODEL.into(),
            voice: VOICE.into(),
            speed: super::script::DEFAULT_SPEED,
        }
    }
}

/// Something that reads text aloud; tests answer with a tone.
pub trait Speak: Sync {
    /// MP3 bytes of `text` read by `voice`.
    fn speak(&self, model: &str, voice: &str, text: &str) -> Result<Vec<u8>, String>;
    /// Dollars per character read, when the provider says.
    fn price_per_char(&self, _model: &str) -> Option<f64> {
        None
    }
}

/// OpenRouter's speech endpoint on the key the app already keeps for transcription and review.
pub struct OpenRouter {
    client: reqwest::blocking::Client,
    key: String,
}

impl OpenRouter {
    pub fn new(key: String) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(180))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client, key })
    }

    /// The OpenRouter key in the keychain (or `OPENROUTER_API_KEY`), if one is set.
    pub fn from_keychain() -> Option<Self> {
        let config = crate::transcript::settings::config_dir();
        let (key, _) = crate::transcript::settings::openrouter_key(&config)?;
        Self::new(key).ok()
    }
}

fn refusal(status: u16, body: &str) -> String {
    match status {
        401 | 403 => "OpenRouter refused the key: check it in transcription settings".into(),
        402 => "The OpenRouter credits are used up: top them up at openrouter.ai".into(),
        429 => "OpenRouter is busy: try again in a minute".into(),
        _ => {
            let detail: String = body.chars().take(300).collect();
            format!("OpenRouter answered HTTP {status}: {detail}")
        }
    }
}

impl Speak for OpenRouter {
    fn speak(&self, model: &str, voice: &str, text: &str) -> Result<Vec<u8>, String> {
        let response = self
            .client
            .post(SPEECH_URL)
            .bearer_auth(&self.key)
            .header("X-Title", "AeroEdits")
            .json(&serde_json::json!({
                "model": model,
                "input": text,
                "voice": voice,
                "response_format": "mp3",
            }))
            .send()
            .map_err(|e| format!("Could not reach OpenRouter: {e}"))?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            let body = response.text().unwrap_or_default();
            return Err(refusal(status, &body));
        }
        let bytes = response.bytes().map_err(|e| e.to_string())?;
        if bytes.len() < 256 {
            return Err(format!("{model} sent no audio"));
        }
        Ok(bytes.to_vec())
    }

    fn price_per_char(&self, model: &str) -> Option<f64> {
        let catalog: serde_json::Value = self.client.get(MODELS_URL).send().ok()?.json().ok()?;
        catalog["data"]
            .as_array()?
            .iter()
            .find(|m| m["id"].as_str() == Some(model))?["pricing"]["prompt"]
            .as_str()?
            .parse()
            .ok()
    }
}

/// The narration as the voice should read it. A dollar amount under one is said in cents: the
/// voice read "$0.2085" as "zero twenty and zero eighty-five", and "$0.07" is "7 cents" to the
/// ear.
pub fn spoken_text(narration: &str) -> String {
    let re = regex::Regex::new(r"\$0\.(\d+)\b").expect("valid");
    re.replace_all(narration, |caps: &regex::Captures| cents(&caps[1]))
        .into_owned()
}

/// "07" → "7 cents", "2085" → "20.85 cents".
fn cents(digits: &str) -> String {
    let head: String = digits.chars().take(2).collect();
    let whole = format!("{head:0<2}").parse::<u32>().unwrap_or(0);
    let rest = digits.get(2..).unwrap_or("").trim_end_matches('0');
    if rest.is_empty() {
        format!("{whole} cents")
    } else {
        format!("{whole}.{rest} cents")
    }
}

/// One voiced scene, in the narration's time.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VoicedScene {
    pub start_us: u64,
    pub end_us: u64,
}

/// Something worth a listen: a name or number the transcription didn't hear as written.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// 1-based, as the script reads.
    pub scene: usize,
    /// `model name` or `number`.
    pub kind: String,
    pub expected: String,
}

/// `faceless/voice.json`: the voice stage's file.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub model: String,
    pub voice: String,
    pub speed: f64,
    /// The joined narration, relative to `faceless/`.
    pub narration: String,
    pub duration_us: u64,
    pub scenes: Vec<VoicedScene>,
    /// Scenes paid for this time (the rest were kept).
    pub generated: usize,
    /// What they cost, when OpenRouter's price is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// The narration's asset on the timeline and its sound key (`<asset>.<stream>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<String>,
    /// Words the transcription heard.
    #[serde(default)]
    pub heard_words: usize,
    #[serde(default)]
    pub issues: Vec<Issue>,
    /// The person listened and the names and numbers are right.
    #[serde(default)]
    pub accepted: bool,
    /// Measured length against the format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_problem: Option<String>,
    /// What the voice stage read when this narration was started (the reviewed script's
    /// fingerprint). Acceptance and the stage hold only while it is still current.
    #[serde(default)]
    pub inputs: String,
    pub at: String,
}

pub fn folder(root: &Path) -> PathBuf {
    root.join(super::DIR).join(FOLDER)
}

pub fn path(root: &Path) -> PathBuf {
    root.join(super::DIR).join("voice.json")
}

pub fn load(root: &Path) -> Option<Voice> {
    crate::files::load_json(&path(root))
}

pub fn save(root: &Path, voice: &Voice) -> Result<(), String> {
    crate::files::save_json(&path(root), voice)
}

fn hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .take(10)
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn fingerprint(settings: &VoiceSettings, spoken: &str) -> String {
    hash(&format!("{}|{}|{spoken}", settings.model, settings.voice))
}

/// The productions being voiced now (their voice folders).
static VOICING: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

/// Holds a production's voice folder while a job writes to it; a second job is refused
/// (GAP-037) rather than mixing its takes with the first's.
struct Job(PathBuf);

impl Job {
    fn start(dir: &Path) -> Result<Self, String> {
        let mut voicing = VOICING.lock().unwrap_or_else(|e| e.into_inner());
        if voicing.iter().any(|d| d == dir) {
            return Err(
                "This video's narration is already being made: wait for it to finish".into(),
            );
        }
        voicing.push(dir.to_path_buf());
        Ok(Self(dir.to_path_buf()))
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        let mut voicing = VOICING.lock().unwrap_or_else(|e| e.into_inner());
        voicing.retain(|d| d != &self.0);
    }
}

/// Voices the scenes not voiced yet (or changed, or listed in `redo`, 0-based), plays each at
/// the channel's pace and joins them into one narration file. What was paid for is kept as
/// soon as it arrives, so a failure part way loses nothing.
pub fn make(
    root: &Path,
    script: &Script,
    settings: &VoiceSettings,
    speak: &dyn Speak,
    redo: &[usize],
    progress: &dyn Fn(f64, &str),
) -> Result<Voice, String> {
    if script.scenes.is_empty() {
        return Err("The script has no scenes".into());
    }
    if script.scenes.len() > 200 {
        return Err("A script can have at most 200 scenes".into());
    }
    if let Some(&bad) = redo.iter().find(|&&i| i >= script.scenes.len()) {
        return Err(format!(
            "There is no scene {} (the script has {})",
            bad + 1,
            script.scenes.len()
        ));
    }
    let dir = folder(root);
    // One voice job per production: the takes, the manifest and the paced files are shared.
    let _job = Job::start(&dir)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let manifest_path = dir.join(MANIFEST);
    let mut manifest: BTreeMap<String, String> =
        crate::files::load_json(&manifest_path).unwrap_or_default();
    let spoken: Vec<String> = script
        .scenes
        .iter()
        .map(|s| spoken_text(s.narration.trim()))
        .collect();
    if let Some(i) = spoken.iter().position(|s| s.is_empty()) {
        return Err(format!("Scene {} has no narration", i + 1));
    }
    if let Some(i) = spoken
        .iter()
        .position(|s| s.chars().count() > MAX_SCENE_CHARS)
    {
        return Err(format!(
            "Scene {} is longer than {MAX_SCENE_CHARS} characters: split it",
            i + 1
        ));
    }
    let raw = |i: usize| dir.join(format!("raw{i}.mp3"));
    let prints: Vec<String> = spoken.iter().map(|s| fingerprint(settings, s)).collect();
    let todo: Vec<usize> = (0..spoken.len())
        .filter(|&i| {
            redo.contains(&i)
                || manifest.get(&i.to_string()) != Some(&prints[i])
                || !raw(i).is_file()
        })
        .collect();
    let total = todo.len();
    let mut failures = Vec::new();
    for (batch, chunk) in todo.chunks(AT_ONCE).enumerate() {
        progress(
            (batch * AT_ONCE) as f64 / total.max(1) as f64 * 0.8,
            &format!("Voicing scenes {} of {total}", batch * AT_ONCE + 1),
        );
        let results: Vec<(usize, Result<Vec<u8>, String>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|&i| {
                    let text = &spoken[i];
                    (
                        i,
                        scope.spawn(move || speak.speak(&settings.model, &settings.voice, text)),
                    )
                })
                .collect();
            handles
                .into_iter()
                .map(|(i, h)| {
                    (
                        i,
                        h.join()
                            .unwrap_or_else(|_| Err("The voice call stopped".into())),
                    )
                })
                .collect()
        });
        for (i, result) in results {
            match result {
                Ok(bytes) => {
                    std::fs::write(raw(i), bytes).map_err(|e| format!("Saving scene {i}: {e}"))?;
                    manifest.insert(i.to_string(), prints[i].clone());
                }
                Err(error) => failures.push(format!("scene {}: {error}", i + 1)),
            }
        }
        crate::files::save_json(&manifest_path, &manifest)?;
    }
    if !failures.is_empty() {
        return Err(format!(
            "{} of {total} scenes were not voiced ({}); the rest are kept, so trying again pays only for these",
            failures.len(),
            failures.join("; ")
        ));
    }
    progress(0.85, "Setting the pace");
    let mut parts = Vec::with_capacity(spoken.len());
    let mut scenes = Vec::with_capacity(spoken.len());
    let mut cursor = LEAD_US;
    for i in 0..spoken.len() {
        let paced = dir.join(format!("vo{i}.wav"));
        crate::media::ffmpeg::retime_audio_wav(&raw(i), settings.speed, &paced)?;
        let length = crate::media::ffmpeg::duration_us(&paced)?;
        scenes.push(VoicedScene {
            start_us: cursor,
            end_us: cursor + length,
        });
        parts.push((paced, cursor));
        cursor += length + PADDING_US;
    }
    progress(0.92, "Joining the scenes");
    // Named by the takes' own audio, not the words asked for: a take voiced again from the
    // same words is a new narration, with its own asset, transcript and listen.
    let mut takes = Sha256::new();
    for i in 0..spoken.len() {
        let bytes = std::fs::read(raw(i)).map_err(|e| format!("Reading scene {i}: {e}"))?;
        takes.update(Sha256::digest(&bytes));
    }
    takes.update(format!("|{}|{LEAD_US}|{PADDING_US}", settings.speed).as_bytes());
    let name = format!(
        "narration-{}.wav",
        takes
            .finalize()
            .iter()
            .take(10)
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let joined = dir.join(&name);
    let assembled = crate::media::ffmpeg::assemble_audio_wav(&parts, cursor, &joined);
    for (paced, _) in &parts {
        let _ = std::fs::remove_file(paced);
    }
    assembled?;
    let chars: usize = todo.iter().map(|&i| spoken[i].chars().count()).sum();
    let cost_usd = if todo.is_empty() {
        Some(0.0)
    } else {
        speak
            .price_per_char(&settings.model)
            .map(|price| price * chars as f64)
    };
    Ok(Voice {
        model: settings.model.clone(),
        voice: settings.voice.clone(),
        speed: settings.speed,
        narration: format!("{FOLDER}/{name}"),
        duration_us: cursor,
        scenes,
        generated: todo.len(),
        cost_usd,
        at: chrono::Utc::now().to_rfc3339(),
        ..Default::default()
    })
}

/// The length the format wants, or why the narration misses it.
pub fn length_problem(duration_us: u64, format: super::Format, words: usize) -> Option<String> {
    let (low, high) = super::script::seconds(format);
    let seconds = duration_us as f64 / 1e6;
    if seconds >= low && seconds <= high {
        return None;
    }
    let pace = words as f64 / seconds.max(1.0);
    Some(format!(
        "The narration runs {seconds:.0} s, outside the {} format's {low:.0}-{high:.0} s. It \
         was spoken at {pace:.2} words a second: write the script to about {:.0}-{:.0} words.",
        format.label(),
        low * pace,
        high * pace
    ))
}

const DIGITS: [&str; 10] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
];

/// Words as compared: lowercased, thousands groups joined ("1,048,576" or "1 048 576"),
/// spelled letters joined ("G P T" → "gpt"), digit words as digits, and the "point" of a
/// decimal dropped.
fn tokens(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    let mut joined = String::with_capacity(chars.len());
    for (i, &c) in chars.iter().enumerate() {
        let group = (c == ',' || c == ' ')
            && i > 0
            && chars[i - 1].is_ascii_digit()
            && chars
                .get(i + 1..i + 4)
                .is_some_and(|g| g.iter().all(char::is_ascii_digit))
            && !chars.get(i + 4).is_some_and(char::is_ascii_digit);
        if !group {
            joined.push(c);
        }
    }
    let mut raw: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut digit = false;
    for c in joined.chars() {
        let is_digit = c.is_ascii_digit();
        let is_letter = c.is_alphabetic();
        if (is_digit || is_letter) && (current.is_empty() || digit == is_digit) {
            current.push(c);
            digit = is_digit;
        } else {
            if !current.is_empty() {
                raw.push(std::mem::take(&mut current));
            }
            if is_digit || is_letter {
                current.push(c);
                digit = is_digit;
            }
        }
    }
    if !current.is_empty() {
        raw.push(current);
    }
    let mut out: Vec<String> = Vec::new();
    let mut letters = String::new();
    for token in raw {
        if token.chars().count() == 1 && token.chars().all(char::is_alphabetic) {
            letters.push_str(&token);
            continue;
        }
        if !letters.is_empty() {
            out.push(std::mem::take(&mut letters));
        }
        match DIGITS.iter().position(|d| *d == token) {
            Some(n) => out.push(n.to_string()),
            None => out.push(token),
        }
    }
    if !letters.is_empty() {
        out.push(letters);
    }
    let out = spelled_numbers(out);
    let is_num = |s: &String| s.chars().all(|c| c.is_ascii_digit());
    (0..out.len())
        .filter(|&i| {
            !(out[i] == "point"
                && i > 0
                && i + 1 < out.len()
                && is_num(&out[i - 1])
                && is_num(&out[i + 1]))
        })
        .map(|i| out[i].clone())
        .collect()
}

/// Joins a spelled number into digits, as transcription writes them: "ninety 1" → "91",
/// "three hundred and sixty" → "360", "one thousand" → "1000". Digits said one by one
/// ("1 2") stay apart.
fn spelled_numbers(tokens: Vec<String>) -> Vec<String> {
    #[derive(Clone, Copy, PartialEq)]
    enum Last {
        None,
        Unit,
        Tens,
        Hundred,
        Scale,
        And,
    }
    const TEENS: [&str; 10] = [
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 8] = [
        "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    enum Word {
        Unit(u64),
        Tens(u64),
        Hundred,
        Scale(u64),
    }
    let word = |t: &str| -> Option<Word> {
        if t.len() == 1 && t.chars().all(|c| c.is_ascii_digit()) {
            return t.parse().ok().map(Word::Unit);
        }
        if let Some(i) = TEENS.iter().position(|w| *w == t) {
            return Some(Word::Unit(10 + i as u64));
        }
        if let Some(i) = TENS.iter().position(|w| *w == t) {
            return Some(Word::Tens(20 + 10 * i as u64));
        }
        match t {
            "hundred" => Some(Word::Hundred),
            "thousand" => Some(Word::Scale(1_000)),
            "million" => Some(Word::Scale(1_000_000)),
            "billion" => Some(Word::Scale(1_000_000_000)),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(tokens.len());
    // The number being read: (total of finished scales, current group, last kind, words).
    let (mut total, mut current, mut last, mut words) = (0u64, 0u64, Last::None, 0usize);
    let flush = |out: &mut Vec<String>, total: &mut u64, current: &mut u64, words: &mut usize| {
        if *words > 0 {
            out.push((*total + *current).to_string());
        }
        (*total, *current, *words) = (0, 0, 0);
    };
    for (i, token) in tokens.iter().enumerate() {
        let next_is_number = tokens.get(i + 1).is_some_and(|t| word(t).is_some());
        if token == "and" && matches!(last, Last::Hundred | Last::Scale) && next_is_number {
            last = Last::And;
            continue;
        }
        let fits = |w: &Word, last: Last| match w {
            Word::Unit(n) => match last {
                Last::None | Last::Hundred | Last::Scale | Last::And => true,
                Last::Tens => *n < 10,
                Last::Unit => false,
            },
            Word::Tens(_) => matches!(last, Last::None | Last::Hundred | Last::Scale | Last::And),
            Word::Hundred => matches!(last, Last::None | Last::Unit),
            Word::Scale(_) => last != Last::And,
        };
        let Some(w) = word(token) else {
            flush(&mut out, &mut total, &mut current, &mut words);
            last = Last::None;
            out.push(token.clone());
            continue;
        };
        if !fits(&w, last) {
            flush(&mut out, &mut total, &mut current, &mut words);
            last = Last::None;
        }
        match w {
            Word::Unit(n) => (current, last) = (current + n, Last::Unit),
            Word::Tens(n) => (current, last) = (current + n, Last::Tens),
            Word::Hundred => (current, last) = (current.max(1) * 100, Last::Hundred),
            Word::Scale(n) => {
                (total, current, last) = (total + current.max(1) * n, 0, Last::Scale);
            }
        }
        words += 1;
    }
    flush(&mut out, &mut total, &mut current, &mut words);
    out
}

fn contains(heard: &[String], wanted: &[String]) -> bool {
    !wanted.is_empty() && heard.windows(wanted.len()).any(|w| w == wanted)
}

/// Model names ("GPT-5", "o3", "Llama 4") and numbers ("40%", "1,048,576", "$0.07") a scene
/// says, as written.
fn checked_terms(narration: &str) -> Vec<(&'static str, String)> {
    let model = regex::Regex::new(r"\b[A-Za-z][A-Za-z0-9]*-?\d+(?:[.\-]\d+)*\b").expect("valid");
    let number = regex::Regex::new(r"\d+(?:,\d{3})*(?:\.\d+)?%?").expect("valid");
    let mut terms: Vec<(&'static str, String)> = Vec::new();
    for m in model.find_iter(narration) {
        terms.push(("model name", m.as_str().to_string()));
    }
    for m in number.find_iter(narration) {
        let before = narration[..m.start()].chars().last();
        if before.is_some_and(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '_')) {
            continue;
        }
        let dollar = before == Some('$');
        let text = m.as_str();
        terms.push((
            "number",
            if dollar {
                format!("${text}")
            } else {
                text.to_string()
            },
        ));
    }
    let mut seen = std::collections::HashSet::new();
    terms.retain(|t| seen.insert(t.clone()));
    terms
}

/// Names and numbers the transcription didn't hear as the script wrote them. `words`: what
/// was heard, with source times in the narration's clock.
pub fn heard_issues(script: &Script, voice: &Voice, words: &[(String, u64, u64)]) -> Vec<Issue> {
    const SLACK_US: u64 = 150_000;
    let mut issues = Vec::new();
    for (index, (scene, span)) in script.scenes.iter().zip(&voice.scenes).enumerate() {
        let said: Vec<&str> = words
            .iter()
            .filter(|(_, start, _)| {
                *start + SLACK_US >= span.start_us && *start <= span.end_us + SLACK_US
            })
            .map(|(w, _, _)| w.as_str())
            .collect();
        let heard = tokens(&said.join(" "));
        for (kind, expected) in checked_terms(&scene.narration) {
            let bare = expected.trim_start_matches('$');
            let mut forms = vec![bare.to_string()];
            if let Some(digits) = bare
                .strip_prefix("0.")
                .filter(|_| expected.starts_with('$'))
            {
                forms.push(cents(digits).trim_end_matches(" cents").to_string());
            }
            if !forms.iter().any(|f| contains(&heard, &tokens(f))) {
                issues.push(Issue {
                    scene: index + 1,
                    kind: kind.into(),
                    expected,
                });
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faceless::script::Scene;

    #[test]
    fn prices_under_a_dollar_are_read_in_cents() {
        assert_eq!(
            spoken_text("It costs $0.07 a call"),
            "It costs 7 cents a call"
        );
        assert_eq!(spoken_text("$0.2085 each"), "20.85 cents each");
        assert_eq!(spoken_text("$0.5"), "50 cents");
        assert_eq!(spoken_text("$4.50 and $10"), "$4.50 and $10");
    }

    #[test]
    fn heard_words_are_compared_loosely() {
        assert_eq!(tokens("G P T 5"), ["gpt", "5"]);
        assert_eq!(tokens("1,048,576 tokens"), ["1048576", "tokens"]);
        assert_eq!(tokens("1 048 576"), ["1048576"]);
        assert_eq!(tokens("seventy nine point one"), ["79", "1"]);
        assert_eq!(tokens("ninety-one percent"), ["91", "percent"]);
        assert_eq!(tokens("three hundred and sixty tries"), ["360", "tries"]);
        assert_eq!(tokens("one thousand rooms"), ["1000", "rooms"]);
        assert_eq!(tokens("1,000 rooms"), ["1000", "rooms"]);
        assert_eq!(tokens("two million and five"), ["2000005"]);
        assert_eq!(tokens("one two three"), ["1", "2", "3"]);
        assert_eq!(tokens("bread and butter"), ["bread", "and", "butter"]);
        assert_eq!(tokens("79 point 1%"), ["79", "1"]);
        assert_eq!(tokens("79.1%"), ["79", "1"]);
        assert_eq!(tokens("GPT-5.1"), ["gpt", "5", "1"]);
    }

    fn voiced(n: usize) -> Voice {
        Voice {
            scenes: (0..n as u64)
                .map(|i| VoicedScene {
                    start_us: i * 10_000_000,
                    end_us: i * 10_000_000 + 9_000_000,
                })
                .collect(),
            ..Default::default()
        }
    }

    fn heard(text: &str, at: u64) -> Vec<(String, u64, u64)> {
        text.split_whitespace()
            .enumerate()
            .map(|(i, w)| {
                (
                    w.to_string(),
                    at + i as u64 * 300_000,
                    at + i as u64 * 300_000 + 200_000,
                )
            })
            .collect()
    }

    #[test]
    fn misheard_names_and_numbers_are_flagged_by_scene() {
        let script = Script {
            scenes: vec![
                Scene {
                    narration: "GPT-5 scored 79.1% and costs $0.07.".into(),
                    ..Default::default()
                },
                Scene {
                    narration: "Llama 4 read 1,048,576 tokens.".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let mut words = heard("GPT 5 scored 79.1% and costs 7 cents.", 0);
        words.extend(heard("Lama for read 1,048,576 tokens.", 10_000_000));
        let issues = heard_issues(&script, &voiced(2), &words);
        assert_eq!(
            issues,
            [Issue {
                scene: 2,
                kind: "number".into(),
                expected: "4".into()
            }]
        );
    }

    #[test]
    fn the_length_is_measured_against_the_format() {
        use crate::faceless::Format;
        assert!(length_problem(45_000_000, Format::Short, 130).is_none());
        let long = length_problem(62_600_000, Format::Short, 168).unwrap();
        assert!(long.contains("63 s"), "{long}");
        assert!(long.contains("2.68 words a second"), "{long}");
    }

    /// GAP-004: a scene voiced again from the same words but coming back as different audio
    /// is a new narration, though nothing in the script changed.
    #[test]
    fn a_take_voiced_again_is_a_new_narration() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let script = Script {
            scenes: vec![Scene {
                narration: "The same words both times.".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let tone = Tone(Default::default());
        let settings = VoiceSettings::default();
        let first = make(root, &script, &settings, &tone, &[], &|_, _| {}).unwrap();
        let again = make(root, &script, &settings, &tone, &[0], &|_, _| {}).unwrap();
        assert_eq!(again.generated, 1);
        assert_ne!(again.narration, first.narration, "new audio, new narration");
        assert!(
            root.join("faceless").join(&first.narration).is_file(),
            "kept for undo"
        );
        let kept = make(root, &script, &settings, &tone, &[], &|_, _| {}).unwrap();
        assert_eq!((kept.generated, kept.narration), (0, again.narration));
    }

    /// GAP-037: while one job voices a production, another for the same production is refused
    /// before it pays for anything; once the first ends, voicing works again.
    #[test]
    fn one_voice_job_at_a_time_per_production() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        struct Slow {
            tone: Tone,
            started: std::sync::mpsc::Sender<()>,
            go: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
        }
        impl Speak for Slow {
            fn speak(&self, model: &str, voice: &str, text: &str) -> Result<Vec<u8>, String> {
                let _ = self.started.send(());
                let _ = self.go.lock().unwrap().recv();
                self.tone.speak(model, voice, text)
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let script = |text: &str| Script {
            scenes: vec![Scene {
                narration: text.into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let settings = VoiceSettings::default();
        let (started_tx, started) = std::sync::mpsc::channel();
        let (go, go_rx) = std::sync::mpsc::channel();
        let slow = Slow {
            tone: Tone(Default::default()),
            started: started_tx,
            go: std::sync::Mutex::new(go_rx),
        };
        let other = Tone(Default::default());
        std::thread::scope(|scope| {
            let first =
                scope.spawn(|| make(root, &script("Job A."), &settings, &slow, &[], &|_, _| {}));
            started.recv().unwrap();
            let second = make(root, &script("Job B."), &settings, &other, &[], &|_, _| {});
            assert!(second.unwrap_err().contains("already being made"));
            assert_eq!(
                other.0.load(std::sync::atomic::Ordering::SeqCst),
                0,
                "B paid for nothing"
            );
            go.send(()).unwrap();
            first.join().unwrap().unwrap();
        });
        let after = make(root, &script("Job B."), &settings, &other, &[], &|_, _| {}).unwrap();
        assert_eq!(after.generated, 1);
    }

    /// A tone for every scene, to test the rest without paying. Each call is pitched a little
    /// higher than the last: a provider's new take of the same words is other audio.
    struct Tone(std::sync::atomic::AtomicUsize);

    impl Speak for Tone {
        fn speak(&self, _: &str, _: &str, text: &str) -> Result<Vec<u8>, String> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
            let file = dir.path().join("t.mp3");
            let pitch = 440 + 20 * self.0.load(std::sync::atomic::Ordering::SeqCst);
            let seconds = (text.split_whitespace().count() as f64 / 2.4).max(0.5);
            let status = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path()?)
                .args(["-v", "error", "-f", "lavfi", "-i"])
                .arg(format!("sine=frequency={pitch}:duration={seconds}"))
                .arg(&file)
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err("ffmpeg failed".into());
            }
            std::fs::read(file).map_err(|e| e.to_string())
        }
    }

    #[test]
    fn only_changed_scenes_are_voiced_again_and_the_pace_shortens_them() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let scene = |text: &str| Scene {
            narration: text.into(),
            ..Default::default()
        };
        let mut script = Script {
            scenes: vec![
                scene("One two three four five six seven eight nine ten eleven twelve."),
                scene("A second scene of some words."),
            ],
            ..Default::default()
        };
        let tone = Tone(Default::default());
        let settings = VoiceSettings::default();
        let first = make(root, &script, &settings, &tone, &[], &|_, _| {}).unwrap();
        assert_eq!(first.generated, 2);
        assert_eq!(tone.0.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(first.scenes[0].start_us, LEAD_US);
        // 5 s of tone at 1.2x is about 4.17 s.
        let length = first.scenes[0].end_us - first.scenes[0].start_us;
        assert!((4_000_000..4_400_000).contains(&length), "{length}");
        assert_eq!(
            first.scenes[1].start_us,
            first.scenes[0].end_us + PADDING_US
        );
        assert_eq!(first.duration_us, first.scenes[1].end_us + PADDING_US);
        assert!(root.join("faceless").join(&first.narration).is_file());
        // Unchanged: nothing is paid for.
        let again = make(root, &script, &settings, &tone, &[], &|_, _| {}).unwrap();
        assert_eq!(
            (again.generated, again.narration.clone()),
            (0, first.narration.clone())
        );
        // One scene changed, one asked for again.
        script.scenes[1].narration = "A different second scene.".into();
        let changed = make(root, &script, &settings, &tone, &[0], &|_, _| {}).unwrap();
        assert_eq!(changed.generated, 2);
        assert_ne!(changed.narration, first.narration);
        let fourth = make(root, &script, &settings, &tone, &[], &|_, _| {}).unwrap();
        assert_eq!(fourth.generated, 0);
        assert_eq!(fourth.narration, changed.narration);
        assert!(make(root, &script, &settings, &tone, &[5], &|_, _| {}).is_err());
    }
}
