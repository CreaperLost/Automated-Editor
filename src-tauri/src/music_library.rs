//! The music library (SND-3): built-in CC0 tracks and the user's own, each tagged by mood and
//! carrying its licence. Music must never need credit (George, 21 Oct): the library refuses a
//! track whose licence asks for credit, forbids monetised use or isn't known, and `check`
//! flags such music anywhere in an edit.
//!
//! The built-in tracks are too big to ship inside the app (nearly forty songs), so each is listed
//! with its file on the Free Music Archive, its size and SHA-256, downloaded the first time it
//! is played or placed and kept in `music/built-in`. Where each came from, and what was turned
//! down, is in `assets/music/LICENSES.md`. The user's own go in `music/mine` with `mine.json`.
//!
//! Placing a track fills a stretch of the timeline on a music track (repeating it when the
//! stretch is longer), with fades, and sets the music under speech when the project has no
//! level for it, so a bed never drowns the voice. One undo step; nothing else moves.
use crate::project::revision::EditDocument;
use crate::sequence::{Asset, Licence, LicenceKind, Role};
use crate::sound_library::tidy_tags;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The name of the track music goes on when no music track is free.
pub const TRACK_NAME: &str = "Music";
const MINE_FILE: &str = "mine.json";
const MAX_MINE: usize = 500;
/// A track's length at most (an hour); at least ten seconds, shorter is a sound effect.
const MAX_TRACK_US: u64 = 3_600_000_000;
const MIN_TRACK_US: u64 = 10_000_000;
/// The level placed music is brought to before the mix (integrated LUFS), so tracks sit alike
/// when the project plays music as placed.
const REFERENCE_LUFS: f32 = -16.0;
/// How far under the speech music sits when the project has no level for it (the Audio
/// panel's default).
pub const DEFAULT_UNDER_SPEECH_DB: f32 = 18.0;
/// Fades: in at the start of a bed, out at its end, and where a repeat begins again.
const FADE_IN_US: u64 = 1_000_000;
const FADE_OUT_US: u64 = 2_500_000;
const REPEAT_FADE_US: u64 = 400_000;

struct Builtin {
    id: &'static str,
    /// Who made it, as their FMA page names them.
    artist: &'static str,
    name: &'static str,
    tags: &'static [&'static str],
    duration_ms: u32,
    /// Where it starts playing: past a silent intro.
    start_ms: u32,
    /// Integrated loudness of the file, measured once.
    lufs: f32,
    /// The track's FMA page (after `PAGES`), which states its licence.
    page: &'static str,
    /// The file (after `FILES`).
    file: &'static str,
    size: u64,
    sha256: &'static str,
}

const FILES: &str = "https://files.freemusicarchive.org/storage-freemusicarchive-org/";
const PAGES: &str = "https://freemusicarchive.org/music/";

macro_rules! track {
    ($artist:literal, $id:literal, $name:literal, [$($tag:literal),*], $ms:literal, $start:literal,
     $lufs:literal, $page:literal, $file:literal, $size:literal, $sha:literal) => {
        Builtin {
            id: $id,
            artist: $artist,
            name: $name,
            tags: &[$($tag),*],
            duration_ms: $ms,
            start_ms: $start,
            lufs: $lufs,
            page: $page,
            file: $file,
            size: $size,
            sha256: $sha,
        }
    };
}

/// The built-in tracks, by HoliznaCC0 and Komiku (CC0 1.0, checked on each track's page).
static BUILTIN: [Builtin; 38] = [
    track!(
        "HoliznaCC0",
        "calm-currents",
        "Calm Currents",
        ["calm", "focus", "lofi", "relaxed"],
        147648,
        0,
        -15.0,
        "holiznacc0/public-domain-lofi/calm-currents-lofi-relax-calm/",
        "tracks/4rKapZUMNnNSPAOvpjlfSH6B5Ib8rgEWdvjnM7C6.mp3",
        5907043,
        "2dcb297376cfe0cb8a547ce74e4d10f406649e007ff01d6e2edb573996d23dc4"
    ),
    track!(
        "HoliznaCC0",
        "waiting-around",
        "Waiting Around",
        ["calm", "focus", "lofi"],
        151776,
        0,
        -10.9,
        "holiznacc0/public-domain-lofi/waiting-around-lofi-calm/",
        "tracks/aVNvUkfJVw1NI9zHSC3I8d760YbwCt31a3Wi6Ydl.mp3",
        6072163,
        "72abc343c2a33388d59f8ac15c6d0f5961cedb43a8db06e599b1fb3dfb28311d"
    ),
    track!(
        "HoliznaCC0",
        "tokyo-sunset",
        "Tokyo Sunset",
        ["calm", "soft", "lofi", "peaceful"],
        139008,
        0,
        -15.5,
        "holiznacc0/public-domain-lofi/tokyo-sunset-lofi-peaceful-soft/",
        "tracks/Xnd9Hr5AVzB68IlWcImKtXPlwCePD2G2m8ZFSVj4.mp3",
        5561443,
        "83b9208ab781af9b2da86e6885c4ec0fc1270457848609a1f96267539518fb9b"
    ),
    track!(
        "HoliznaCC0",
        "into-the-mist",
        "Into The Mist",
        ["calm", "focus", "lofi", "relaxed"],
        129576,
        0,
        -13.1,
        "holiznacc0/public-domain-lofi/into-the-mist-lofi-calm-relaxed/",
        "tracks/cI5iv9Xga0OOvGsS7mGZn9BlTshZdzAqLW7bmArA.mp3",
        5184163,
        "51abc671658f942ef54ad7340390e2a7e7afa7b2b7a9a223aafb955b27cb3074"
    ),
    track!(
        "HoliznaCC0",
        "wetlands",
        "Wetlands",
        ["calm", "focus", "lofi", "relaxed"],
        163968,
        0,
        -17.8,
        "holiznacc0/public-domain-lofi/wetlands-lofi-calm-relaxed/",
        "tracks/4ZpahEeu0NHfAIjJXrlAvVUg8C4y71ycbjonRGke.mp3",
        6559843,
        "81d8e723ebc2dc92eec354261b3df983b3a418981fbae6fc3476ae73a1979ca8"
    ),
    track!(
        "HoliznaCC0",
        "peaceful-drift",
        "Peaceful Drift",
        ["calm", "nostalgic", "lofi"],
        128304,
        0,
        -15.8,
        "holiznacc0/public-domain-lofi/peaceful-drift-lofi-nostalgic-calm/",
        "tracks/SQvtLguk6S1VSthv0oXWycoB6ipUS0pt8jzAxxPq.mp3",
        5133283,
        "fc21d359657c8c7d01fbc7a7a28443569d499ebdb893a46761ee1d87bf8b5afb"
    ),
    track!(
        "HoliznaCC0",
        "theta-frequency",
        "Theta Frequency",
        ["calm", "focus", "lofi", "chill"],
        126840,
        0,
        -13.2,
        "holiznacc0/public-domain-lofi/theta-frequency-lofi-chill-calm/",
        "tracks/rSIDyunfJfiKNelwFuwbGKoLj5TO8eHFbdSa1zAb.mp3",
        5074723,
        "4464a327ae98a9cee7072d78eb2c1cd7096816a33f46cbab86129b823607db5a"
    ),
    track!(
        "HoliznaCC0",
        "bubbles",
        "Bubbles",
        ["bright", "happy", "lofi", "relaxed"],
        147648,
        0,
        -13.7,
        "holiznacc0/public-domain-lofi/bubbles-lofi-bright-relaxed/",
        "tracks/zh5usndkyEfVvKOz8dvsrCd1Ga0jfz4xIcLUendD.mp3",
        5907043,
        "73efb557d8cfcca0f5cf3b435d3a0157bf9a702115b55266b8a2529f3997b542"
    ),
    track!(
        "HoliznaCC0",
        "tranquil-mindscape",
        "Tranquil Mindscape",
        ["happy", "bright", "lofi"],
        153552,
        0,
        -13.8,
        "holiznacc0/public-domain-lofi/tranquil-mindscape-lofi-happy-reflection/",
        "tracks/uxGaqYrlWHy1w4jA6fJki0NlNx6xkkFG5BMwI8JN.mp3",
        6143203,
        "488d9693c13e44c5213c0647ee89ea91b21e17d9ac602225a78ee089720134c0"
    ),
    track!(
        "HoliznaCC0",
        "doodles",
        "Doodles",
        ["happy", "bright", "lofi"],
        138336,
        0,
        -10.6,
        "holiznacc0/public-domain-lofi/doodles-lofi-happy-mp3/",
        "tracks/rQGj86J7JXRwu3BRKtB6GIfnwCwI5slkTwfTWqvI.mp3",
        5534605,
        "84db72387b14fa51294ba76a0bb3f72f39172527e5eb7964da8567596561ae72"
    ),
    track!(
        "HoliznaCC0",
        "projector-screen",
        "Projector Screen",
        ["happy", "bright", "lofi"],
        142272,
        0,
        -11.1,
        "holiznacc0/public-domain-lofi/projector-screen-lofi-happy-mp3/",
        "tracks/SrYI6LJrSooBuV1Vz7XocwyKdkoXRFyNpgzn0lI4.mp3",
        5692003,
        "b757435f375fc9290483e6ffabf763598664b1bb37fcd4644712529ba3dd5e04"
    ),
    track!(
        "HoliznaCC0",
        "walking-away",
        "Walking Away",
        ["upbeat", "motivating", "lofi", "peaceful"],
        156048,
        0,
        -13.2,
        "holiznacc0/public-domain-lofi/walking-away-lofi-peaceful-motivating/",
        "tracks/WOtcP3GhbgTD8CuC6sEgpQOEXMyNeXYbTNmHjgN6.mp3",
        6243043,
        "fd8a618a4076b76f02f69414d6beb4ad7ddf3b8c26164085bd6dc1a9fd2f7610"
    ),
    track!(
        "HoliznaCC0",
        "ocean-breeze",
        "Ocean Breeze",
        ["happy", "ukulele", "acoustic", "peaceful"],
        197640,
        0,
        -11.2,
        "holiznacc0/public-domain-lofi/ocean-breeze-lofi-ukulele-peaceful-mp3/",
        "tracks/7ahdWvuUuDCVriuh5RvYsl96eUmAJuaVvC30sqlW.mp3",
        7906723,
        "3dbc9713e17d3e23d942d36510ba046210f69d4e2df4ebb07e7d2ce4c2e6b358"
    ),
    track!(
        "HoliznaCC0",
        "lucid",
        "Lucid",
        ["dreamy", "chill", "lofi"],
        144936,
        0,
        -10.3,
        "holiznacc0/public-domain-lofi/lucid-lofi-dreamy-chill/",
        "tracks/je7RethXWuduCoRV6Gq3w25yDXvxYnnOWt5OGlgv.mp3",
        5798563,
        "7378b49dde3aef61d0f52aba71a06089e8513cf7cf9f7b4c33048cc2d350b05e"
    ),
    track!(
        "HoliznaCC0",
        "wave-maker",
        "Wave Maker",
        ["dreamy", "retro", "lofi"],
        185304,
        0,
        -13.9,
        "holiznacc0/public-domain-lofi/wave-maker-lofi-dreamy-retro/",
        "tracks/SbPJTWMnBmT5yBsXP4jLfp6RsvWLhfOnbw2PxXol.mp3",
        7413283,
        "e525e8c2953e4a10c31ea6ca42449f793f57bf11066a570b52a2389820a782f7"
    ),
    track!(
        "HoliznaCC0",
        "infinite-echoes",
        "Infinite Echoes",
        ["dreamy", "soft", "lofi"],
        140280,
        0,
        -12.1,
        "holiznacc0/public-domain-lofi/infinite-echoes-lofi-dreamy-soft/",
        "tracks/fNTwTe4hAptJKPxgouogjFHY3KUWqInieyDckOSk.mp3",
        5612323,
        "0e526a82107409f9937974100dd176e4f5b6c0af115ce6ad8f68d36dc2fd1702"
    ),
    track!(
        "HoliznaCC0",
        "summer-break",
        "Summer Break",
        ["nostalgic", "lofi"],
        173976,
        0,
        -11.3,
        "holiznacc0/public-domain-lofi/summer-break-lofi-nostalgic-mp3/",
        "tracks/9lO4AYmwyGCp44JI80uCvfg0XgX1NgCvd8EgzKZN.mp3",
        6960205,
        "f6c5c67a3b682ff574f8073e22cb70bef3e5cc502a41ec62783ac70ebbb3159f"
    ),
    track!(
        "HoliznaCC0",
        "roof-tops",
        "Roof Tops",
        ["nostalgic", "chill", "lofi"],
        214464,
        0,
        -14.9,
        "holiznacc0/summer-air-lo-fi/roof-tops/",
        "tracks/2hFTiuqYI7onbLzpi9ldoBP8AYeuStsCUxTqm6vd.mp3",
        8902309,
        "f6e619fbfdc0898c409494d5c810d34d9cd29be2b75bb2d8c197e1edb37155b5"
    ),
    track!(
        "HoliznaCC0",
        "one-night-in-france",
        "One Night In France",
        ["nostalgic", "chill", "lofi"],
        194808,
        0,
        -14.6,
        "holiznacc0/public-domain-lofi/one-night-in-france-lofi-nostalgic-chill/",
        "tracks/hgY8pG5KicULRbUJjUVavqs1h53lNSHdLptPoCwS.mp3",
        7793443,
        "9043a6d5468598f9ff9ec1e70970e9690f324dedea160f2bc266eec9e883d7b0"
    ),
    track!(
        "HoliznaCC0",
        "we-drove-all-night",
        "We Drove All Night",
        ["game", "energetic", "upbeat"],
        157872,
        0,
        -14.5,
        "holiznacc0/we-drove-all-night-game-soundtrack/we-drove-all-night/",
        "tracks/m0FGlg9TFoyweOePWHw8uvhqA5s7rXgR4pScZat4.mp3",
        6562012,
        "7f2f8953c5cb21c4704ec700a2c16f75043f6ceb7d8cfab69e69bc175b6ffaed"
    ),
    track!(
        "HoliznaCC0",
        "side-quest",
        "Side Quest to Nowhere",
        ["game", "upbeat", "playful"],
        237000,
        0,
        -12.7,
        "holiznacc0/we-drove-all-night-game-soundtrack/side-quest-to-nowhere/",
        "tracks/tZN6YhIxHjI7BxY3pmKJHkhAdhuYZOxT6k9wLgRC.mp3",
        9727132,
        "2441af7dcf88bc7f45f182cf30678fb00680c87a614bc139b34aa1fdd40b4885"
    ),
    track!(
        "HoliznaCC0",
        "skill-test",
        "Skill Test",
        ["game", "dark", "trap", "energetic"],
        150840,
        0,
        -10.9,
        "holiznacc0/after-dark/skill-test/",
        "tracks/91zT2eS5mOUpUgqyOcEmxruDr0A8kWM5WUlpF8n6.mp3",
        6108691,
        "f33c559f557cf1158ccbfc9b0dce724f964495039ef4abe3f10b5744f0cbb193"
    ),
    track!(
        "HoliznaCC0",
        "phonk-ish",
        "Phonk ish",
        ["game", "phonk", "energetic", "intense"],
        145560,
        0,
        -14.3,
        "holiznacc0/phonk-aura-farming/phonk-ish/",
        "tracks/RLX0u2cmWIjC0KerQHMzbWSILeGhWLIkxWorpbeQ.mp3",
        5823523,
        "1b9f5f16caf1e3db60a731eb78bc854b049bce5fc89093ee202da6da15966817"
    ),
    track!(
        "HoliznaCC0",
        "ruby-1",
        "Ruby 1",
        ["game", "chiptune", "retro", "playful"],
        128352,
        3700,
        -13.4,
        "holiznacc0/ruby-chiptunes/ruby-1/",
        "tracks/mJ6FK4kyTEtc69eSeLph8dKvWheroaqcChOazEI4.mp3",
        5352550,
        "39c660ab99420dd4e3f7472f7759d4042072e87e53cf4674ff7bfede50aa90d0"
    ),
    track!(
        "Komiku",
        "chill-out-theme",
        "Chill Out Theme",
        ["calm", "chill", "game", "relaxed"],
        187663,
        0,
        -19.2,
        "Komiku/Its_time_for_adventure__vol_2/Komiku_-_Its_time_for_adventure_vol_2_-_02_Chill_Out_Theme/",
        "music/no_curator/Komiku/Its_time_for_adventure__vol_2/Komiku_-_02_-_Chill_Out_Theme.mp3",
        7664273,
        "1ad35c314ff0034bb31332cba7e1926bb028610fa213d2af2e3ed333a07812b1"
    ),
    track!(
        "Komiku",
        "down-the-river",
        "Down the River",
        ["calm", "folk", "acoustic", "peaceful"],
        114546,
        0,
        -17.8,
        "Komiku/Its_time_for_adventure__vol_2/Komiku_-_Its_time_for_adventure_vol_2_-_05_Down_the_river/",
        "music/no_curator/Komiku/Its_time_for_adventure__vol_2/Komiku_-_05_-_Down_the_river.mp3",
        4739603,
        "eabe4f88044e722f6843cd5adcf621500f82f1b95f2a84f32070d08b6d15933b"
    ),
    track!(
        "Komiku",
        "champ-de-tournesol",
        "Champ de Tournesol",
        ["happy", "folk", "bright", "acoustic"],
        117968,
        0,
        -14.4,
        "Komiku/Its_time_for_adventure_/Komiku_-_Its_time_for_adventure_-_03_Champ_de_tournesol/",
        "music/no_curator/Komiku/Its_time_for_adventure_/Komiku_-_03_-_Champ_de_tournesol.mp3",
        4878533,
        "1cd0a1029eadfa6d1bb0b200019e8c4ad761582b78176a2c7750afa5ed5c694e"
    ),
    track!(
        "Komiku",
        "le-grand-village",
        "Le Grand Village",
        ["happy", "game", "folk", "playful"],
        104071,
        0,
        -14.0,
        "Komiku/Its_time_for_adventure_/Komiku_-_Its_time_for_adventure_-_02_Le_Grand_Village/",
        "music/no_curator/Komiku/Its_time_for_adventure_/Komiku_-_02_-_Le_Grand_Village.mp3",
        4322647,
        "f8a64cc1209f9efbbe1a1f437c18d1eed0758ea8a3cf0e0dbc650c87f43b6ab9"
    ),
    track!(
        "Komiku",
        "barque-sur-le-lac",
        "Barque sur le Lac",
        ["calm", "dreamy", "folk", "soft"],
        205949,
        372,
        -15.2,
        "Komiku/Its_time_for_adventure_/Komiku_-_Its_time_for_adventure_-_04_Barque_sur_le_lac/",
        "music/no_curator/Komiku/Its_time_for_adventure_/Komiku_-_04_-_Barque_sur_le_lac.mp3",
        8397749,
        "b6e80f2dffba95a83ad25205d7600a029e747d18bad2d9e1d1557a6c45eeff85"
    ),
    track!(
        "Komiku",
        "childhood-scene",
        "Childhood Scene",
        ["nostalgic", "game", "soft"],
        139964,
        1393,
        -11.9,
        "Komiku/Its_time_for_adventure__vol_3/Komiku_-_Its_time_for_adventure_vol_3_-_01_Childhood_scene/",
        "music/Music_for_Video/Komiku/Its_time_for_adventure__vol_3/Komiku_-_01_-_Childhood_scene.mp3",
        5851521,
        "9f3cea751d43b7054c1f58b5d1c80882980622235d14a4fcc27f4e2164482d7a"
    ),
    track!(
        "Komiku",
        "never-get-old",
        "The Place That Never Get Old",
        ["nostalgic", "game", "dreamy"],
        185547,
        307,
        -17.6,
        "Komiku/Its_time_for_adventure__vol_3/Komiku_-_Its_time_for_adventure_vol_3_-_07_The_place_that_never_get_old/",
        "music/Music_for_Video/Komiku/Its_time_for_adventure__vol_3/Komiku_-_07_-_The_place_that_never_get_old.mp3",
        7674868,
        "bacf01dc7d8fb2ba4935de428d2f3f8ee9cc859e95124b05639490d15e09815f"
    ),
    track!(
        "Komiku",
        "friends-theme",
        "Friends's Theme",
        ["happy", "game", "bright"],
        164022,
        0,
        -13.9,
        "Komiku/Its_time_for_adventure__vol_5/Komiku_-_Its_time_for_adventure_vol_5_-_06_Friendss_theme/",
        "music/Music_for_Video/Komiku/Its_time_for_adventure__vol_5/Komiku_-_06_-_Friendss_theme.mp3",
        6705328,
        "0039d42c0f2bc1b96914c0f7e311613d0c79844ee7b12bec8342bf1243d0c5ba"
    ),
    track!(
        "Komiku",
        "jump-and-shoot",
        "Level 1: Jump and Shoot Stuff 101",
        ["upbeat", "game", "energetic", "synth"],
        154044,
        0,
        -11.1,
        "Komiku/ULTRA_PERSON_VOL1/Komiku_-_ULTRA_PERSON_VOL1_-_02_Level_1_-_Jump_and_shoot_stuff_101/",
        "music/Music_for_Video/Komiku/ULTRA_PERSON_VOL1/Komiku_-_02_-_Level_1__Jump_and_shoot_stuff_101.mp3",
        6287745,
        "b73c5bcaaee8b4bf06bfebd61868cc32cd877e3c17d62c8433c5cefdcbfcfdf9"
    ),
    track!(
        "Komiku",
        "expressway",
        "Level 3: Run on the Expressway",
        ["upbeat", "energetic", "synth", "game"],
        133381,
        0,
        -11.2,
        "Komiku/ULTRA_PERSON_VOL1/Komiku_-_ULTRA_PERSON_VOL1_-_10_Level_3_-_Run_on_the_expressway/",
        "music/Music_for_Video/Komiku/ULTRA_PERSON_VOL1/Komiku_-_10_-_Level_3__Run_on_the_expressway.mp3",
        5461231,
        "e8592734e3de68256f37fa29e76564e762cccae98ebb0386e6fd7a731a63a9dc"
    ),
    track!(
        "Komiku",
        "cat-city",
        "Cat City",
        ["happy", "chiptune", "game", "playful"],
        128026,
        0,
        -21.3,
        "Komiku/Poupis_incredible_adventures_/Komiku_-_Poupis_incredible_adventures__-_15_Cat_City/",
        "music/Music_for_Video/Komiku/Poupis_incredible_adventures_/Komiku_-_15_-_Cat_City.mp3",
        5233714,
        "908ccf9459d5520c62d1aa17969dd0f3f02ea8be2bc8ec5b2c79eedc10653b9b"
    ),
    track!(
        "Komiku",
        "quiet-saturday",
        "Quiet Saturday",
        ["calm", "chiptune", "soft", "peaceful"],
        113188,
        0,
        -22.8,
        "Komiku/Poupis_incredible_adventures_/Komiku_-_Poupis_incredible_adventures__-_57_Quiet_Saturday/",
        "music/Music_for_Video/Komiku/Poupis_incredible_adventures_/Komiku_-_57_-_Quiet_Saturday.mp3",
        4654548,
        "c35d8623b316e0cb544388e4b76232d263e5eb50882d8fc3a5bc97a4505f1719"
    ),
    track!(
        "Komiku",
        "ambient-hope",
        "Ambiant Hope",
        ["calm", "focus", "ambient", "soft"],
        288966,
        230,
        -16.4,
        "Komiku/The_Binge_Watchers__Score_1/Komiku_-_The_Binge_Watchers_-_Score_1_-_10_Ambiant_Hope/",
        "music/Music_for_Video/Komiku/The_Binge_Watchers__Score_1/Komiku_-_10_-_Ambiant_Hope.mp3",
        11651875,
        "0b4fb8f4e7408b2d177c88b9e8b7192e3de8a1f0f01d8df2222915ab0cab1ebd"
    ),
    track!(
        "Komiku",
        "road-1-chill",
        "Road 1 Chill",
        ["chill", "game", "relaxed", "electronic"],
        107546,
        0,
        -15.8,
        "Komiku/Helice_Awesome_Dance_Adventure_/road-1-chill/",
        "tracks/4vWhE7a8gmtWWVblJNHrhgsttzhS5xY62b8VWdXd.mp3",
        4397106,
        "1797ad197cb078456702773799c42593b8cf801658938e29d0fe7350d97cbe37"
    ),
];

/// One track of the library, as the app and agents see it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub duration_us: u64,
    /// Where it starts playing (past a silent intro).
    pub start_us: u64,
    /// The file on disk, played in place; for a built-in track not yet downloaded, where it
    /// will be.
    pub path: String,
    pub built_in: bool,
    /// On disk: a built-in track is downloaded the first time it is used.
    pub downloaded: bool,
    pub licence: Licence,
    /// Integrated loudness, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lufs: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Mine {
    id: String,
    name: String,
    #[serde(default)]
    tags: Vec<String>,
    duration_us: u64,
    file: String,
    licence: Licence,
}

/// The library's folder: the app's, or a test run's own.
pub fn dir() -> PathBuf {
    crate::style::store::config().join("music")
}

fn built_in_path(root: &Path, b: &Builtin) -> PathBuf {
    root.join("built-in").join(format!("{}.mp3", b.id))
}

fn builtin(id: &str) -> Option<&'static Builtin> {
    BUILTIN.iter().find(|b| b.id == id)
}

/// The licence a track must have for the library: usable in a monetised video with no credit.
/// Answers why not.
pub fn refused(licence: &Licence) -> Option<&'static str> {
    match licence.kind {
        k if k.needs_no_credit() => None,
        LicenceKind::CreditRequired => Some("its licence asks for credit"),
        LicenceKind::NonCommercial => Some("its licence forbids monetised use"),
        _ => Some("its licence isn't known"),
    }
}

fn load_mine(root: &Path) -> Vec<Mine> {
    fs::read(root.join("mine").join(MINE_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_mine(root: &Path, mine: &[Mine]) -> Result<(), String> {
    let dir = root.join("mine");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(mine).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, &bytes).map_err(|e| e.to_string())?;
    temp.persist(dir.join(MINE_FILE))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Every track: the built-in ones first, then the user's own, in the order they were added.
pub fn list(root: &Path) -> Vec<Track> {
    let mut tracks: Vec<Track> = BUILTIN
        .iter()
        .map(|b| {
            let path = built_in_path(root, b);
            Track {
                id: b.id.into(),
                name: b.name.into(),
                tags: b.tags.iter().map(|t| t.to_string()).collect(),
                duration_us: b.duration_ms as u64 * 1_000,
                start_us: b.start_ms as u64 * 1_000,
                downloaded: fs::metadata(&path).map(|m| m.len()).ok() == Some(b.size),
                path: path.to_string_lossy().into_owned(),
                built_in: true,
                licence: Licence {
                    kind: LicenceKind::Cc0,
                    source: Some(format!("{}, {PAGES}{}", b.artist, b.page)),
                },
                lufs: Some(b.lufs),
            }
        })
        .collect();
    let mine_dir = root.join("mine");
    tracks.extend(load_mine(root).into_iter().map(|m| Track {
        path: mine_dir.join(&m.file).to_string_lossy().into_owned(),
        downloaded: true,
        id: m.id,
        name: m.name,
        tags: m.tags,
        duration_us: m.duration_us,
        start_us: 0,
        built_in: false,
        licence: m.licence,
        lufs: None,
    }));
    tracks
}

/// The tracks matching `query` (every word in its name, id or tags), all of them when empty.
pub fn search(tracks: Vec<Track>, query: &str) -> Vec<Track> {
    let words: Vec<String> = query
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    tracks
        .into_iter()
        .filter(|t| {
            let hay = format!("{} {} {}", t.id, t.name, t.tags.join(" ")).to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .collect()
}

pub fn find(root: &Path, id: &str) -> Result<Track, String> {
    list(root)
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("No track {id} in the music library"))
}

/// `track` on disk: a built-in one is downloaded (once) and checked against its hash.
pub fn ensure(root: &Path, track: &Track) -> Result<Track, String> {
    if track.downloaded {
        return Ok(track.clone());
    }
    let b = builtin(&track.id).ok_or_else(|| format!("{} is missing", track.path))?;
    let url = format!("{FILES}{}", b.file);
    let bytes = download(&url, b.size)?;
    store(root, b, &bytes)?;
    find(root, &track.id)
}

fn download(url: &str, size: u64) -> Result<Vec<u8>, String> {
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Could not download the track: {e}"))?;
    let mut bytes = Vec::with_capacity(size as usize);
    response
        .take(size + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Could not download the track: {e}"))?;
    Ok(bytes)
}

/// Writes a built-in track's file if it is the one pinned.
fn store(root: &Path, b: &Builtin, bytes: &[u8]) -> Result<(), String> {
    let hash = format!("{:x}", Sha256::digest(bytes));
    if bytes.len() as u64 != b.size || hash != b.sha256 {
        return Err(format!(
            "The downloaded \"{}\" isn't the file the library expects; it was not kept",
            b.name
        ));
    }
    let path = built_in_path(root, b);
    let dir = path.parent().expect("built-in folder");
    fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, bytes).map_err(|e| e.to_string())?;
    temp.persist(&path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Copies a music file of the user's into the library with its tags and licence. A licence
/// that asks for credit, forbids monetised use or isn't known is refused.
pub fn add(
    root: &Path,
    file: &Path,
    name: Option<String>,
    tags: Vec<String>,
    licence: Licence,
) -> Result<Track, String> {
    if let Some(why) = refused(&licence) {
        return Err(format!(
            "The music library only takes tracks free for monetised videos with no credit \
             (CC0, public domain, or your own): {why}"
        ));
    }
    if crate::media_bin::kind_for(file) != Some(crate::sequence::AssetKind::Audio) {
        return Err(format!("{} is not a sound file", file.display()));
    }
    let duration_us = crate::media::ffmpeg::duration_us(file)?;
    if !(MIN_TRACK_US..=MAX_TRACK_US).contains(&duration_us) {
        return Err(
            "A music track runs between ten seconds and an hour; shorter ones go in \
                    Sounds"
                .into(),
        );
    }
    let mut mine = load_mine(root);
    if mine.len() >= MAX_MINE {
        return Err(format!(
            "The library holds {MAX_MINE} tracks of your own at most"
        ));
    }
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "track".into());
    let slug = crate::sound_library::slug(&stem);
    let base = format!("my-{}", if slug.is_empty() { "track" } else { &slug });
    let taken = |id: &str| builtin(id).is_some() || mine.iter().any(|m| m.id == id);
    let id = (1..)
        .map(|n| {
            if n == 1 {
                base.clone()
            } else {
                format!("{base}-{n}")
            }
        })
        .find(|id| !taken(id))
        .unwrap_or(base);
    let ext = file
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "mp3".into());
    let copy = format!("{id}.{ext}");
    let dir = root.join("mine");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::copy(file, dir.join(&copy)).map_err(|e| format!("Could not copy the track: {e}"))?;
    mine.push(Mine {
        id: id.clone(),
        name: name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or(stem),
        tags: tidy_tags(tags),
        duration_us,
        file: copy,
        licence,
    });
    save_mine(root, &mine)?;
    find(root, &id)
}

/// Changes a track of the user's own: its name, tags or licence (never to one refused).
pub fn update(
    root: &Path,
    id: &str,
    name: Option<String>,
    tags: Option<Vec<String>>,
    licence: Option<Licence>,
) -> Result<Track, String> {
    let mut mine = load_mine(root);
    let Some(entry) = mine.iter_mut().find(|m| m.id == id) else {
        return Err(if builtin(id).is_some() {
            "Built-in tracks can't be changed".into()
        } else {
            format!("No track {id} of your own")
        });
    };
    if let Some(why) = licence.as_ref().and_then(refused) {
        return Err(format!(
            "The music library only takes tracks that need no credit: {why}. Remove it instead."
        ));
    }
    if let Some(name) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
        entry.name = name;
    }
    if let Some(tags) = tags {
        entry.tags = tidy_tags(tags);
    }
    if let Some(licence) = licence {
        entry.licence = licence;
    }
    save_mine(root, &mine)?;
    find(root, id)
}

/// Takes a track of the user's out of the library (projects that used it keep their media).
pub fn remove(root: &Path, id: &str) -> Result<(), String> {
    let mut mine = load_mine(root);
    let Some(at) = mine.iter().position(|m| m.id == id) else {
        return Err(format!("No track {id} of your own"));
    };
    let entry = mine.remove(at);
    save_mine(root, &mine)?;
    let _ = fs::remove_file(root.join("mine").join(entry.file));
    Ok(())
}

/// The project's media for `track` (on disk already): the asset holding its file, or a new
/// one imported from it as music with its licence. The second value says whether it is new.
pub fn asset_for(
    document: &EditDocument,
    project_root: &Path,
    track: &Track,
) -> Result<(Asset, bool), String> {
    let ours = Path::new(&track.path);
    let same = |a: &Asset| {
        Path::new(&a.path) == ours
            || dunce::canonicalize(ours)
                .map(|p| Path::new(&a.path) == p)
                .unwrap_or(false)
    };
    if let Some(asset) = document.assets.iter().find(|a| same(a)) {
        return Ok((asset.clone(), false));
    }
    let mut asset = crate::media_bin::import(project_root, ours)?;
    asset.name = track.name.clone();
    for stream in &mut asset.streams {
        stream.role = Role::Music;
    }
    asset.licence = Some(track.licence.clone());
    Ok((asset, true))
}

/// What a placed bed covers and how it was made.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bed {
    pub clip_ids: Vec<String>,
    pub start_us: u64,
    pub end_us: u64,
    /// The music under speech was switched on with this level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub under_speech_db: Option<f32>,
}

/// `document` with `track`'s `asset` (added when new) filling `[at_us, until_us)` (to the end
/// of the video when not given) on a music track free there, or a new one named
/// [`TRACK_NAME`]: repeated when the stretch is longer, faded in, out and at each repeat,
/// brought to a common level, and the music under speech switched on when the project has no
/// level for it. A new track takes an id not in `taken`.
pub fn place(
    document: &EditDocument,
    track: &Track,
    asset: Asset,
    at_us: u64,
    until_us: Option<u64>,
    taken: &[String],
) -> Result<(EditDocument, Bed), String> {
    use crate::sequence::edit::{apply_reserving, SequenceEdit};
    use crate::sequence::SourceRange;
    let mut next = document.clone();
    let asset_id = asset.id.clone();
    let length = asset.duration_us;
    if next.asset(&asset_id).is_none() {
        next.assets.push(asset);
    }
    let video_end = next.duration_us();
    let start = at_us.min(video_end);
    let end = until_us.unwrap_or(video_end).min(video_end);
    if end <= start + 1_000_000 {
        return Err("Music needs at least a second of the video to play under".into());
    }
    let from = track.start_us.min(length.saturating_sub(MIN_TRACK_US / 2));
    let piece = length - from;
    if piece == 0 {
        return Err("The track holds no sound".into());
    }
    let (mut sequence, track_id) = crate::sound_library::free_track(
        &next,
        Role::Music,
        TRACK_NAME,
        start,
        end,
        taken.iter().map(String::as_str),
    )?;
    let mut clip_ids = Vec::new();
    let mut at = start;
    while at < end {
        let len = piece.min(end - at);
        if len < 500_000 && !clip_ids.is_empty() {
            break;
        }
        let before: Vec<String> = sequence.clips().map(|(_, c)| c.id.clone()).collect();
        let (placed, _) = apply_reserving(
            &sequence,
            &next.assets,
            &SequenceEdit::PlaceAsset {
                asset_id: asset_id.clone(),
                at_us: at,
                track_id: Some(track_id.clone()),
                streams: Vec::new(),
                range: Some(SourceRange {
                    start_us: from,
                    end_us: from + len,
                }),
            },
            taken.iter().map(String::as_str),
        )?;
        let id = placed
            .clips()
            .find(|(_, c)| c.asset == asset_id && !before.contains(&c.id))
            .map(|(_, c)| c.id.clone())
            .ok_or("The track was not placed")?;
        sequence = placed;
        clip_ids.push(id);
        at += len;
    }
    let real_end = at.min(end);
    if sequence.duration_us() > video_end {
        return Err("Placing the music would have moved the video".into());
    }
    let gain_db = track.lufs.map(|l| (REFERENCE_LUFS - l).clamp(-12.0, 12.0));
    let last = clip_ids.len() - 1;
    for (i, id) in clip_ids.iter().enumerate() {
        let (sound, _) = apply_reserving(
            &sequence,
            &next.assets,
            &SequenceEdit::SetSound {
                clip_ids: vec![id.clone()],
                gain_db,
                fade_in_us: Some(if i == 0 { FADE_IN_US } else { REPEAT_FADE_US }),
                fade_out_us: Some(if i == last {
                    FADE_OUT_US
                } else {
                    REPEAT_FADE_US
                }),
            },
            taken.iter().map(String::as_str),
        )?;
        sequence = sound;
    }
    next.sequence = sequence;
    let under_speech_db = match next.audio.music_under_speech_db {
        Some(_) => None,
        None => {
            next.audio.music_under_speech_db = Some(DEFAULT_UNDER_SPEECH_DB);
            Some(DEFAULT_UNDER_SPEECH_DB)
        }
    };
    Ok((
        next,
        Bed {
            clip_ids,
            start_us: start,
            end_us: real_end,
            under_speech_db,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_tracks_need_no_credit_and_cover_the_moods() {
        let dir = tempfile::tempdir().unwrap();
        let tracks = list(dir.path());
        assert_eq!(tracks.len(), BUILTIN.len());
        let mut ids: Vec<&str> = BUILTIN.iter().map(|b| b.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), BUILTIN.len(), "ids are unique");
        for (track, b) in tracks.iter().zip(&BUILTIN) {
            assert!(refused(&track.licence).is_none());
            assert!(!track.downloaded);
            assert_eq!(b.sha256.len(), 64);
            assert!(b.size > 1_000_000 && track.duration_us > 60_000_000);
            assert!(track.licence.source.as_deref().unwrap().contains(&format!(
                "{}, https://freemusicarchive.org/music/",
                b.artist
            )));
        }
        for mood in ["calm", "happy", "dreamy", "nostalgic", "game", "upbeat"] {
            assert!(!search(tracks.clone(), mood).is_empty(), "no {mood}");
        }
        assert_eq!(search(tracks, "calm focus").len(), 6);
    }

    #[test]
    fn music_that_needs_credit_or_forbids_monetising_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("song.mp3");
        fs::write(&file, b"x").unwrap();
        for kind in [
            LicenceKind::CreditRequired,
            LicenceKind::NonCommercial,
            LicenceKind::Unknown,
        ] {
            let error = add(
                dir.path(),
                &file,
                None,
                vec![],
                Licence { kind, source: None },
            )
            .unwrap_err();
            assert!(error.contains("no credit"), "{error}");
        }
        assert!(load_mine(dir.path()).is_empty());
    }

    #[test]
    fn a_download_is_kept_only_when_it_matches_its_pin() {
        let dir = tempfile::tempdir().unwrap();
        let b = &BUILTIN[0];
        assert!(store(dir.path(), b, b"not the track").is_err());
        assert!(!built_in_path(dir.path(), b).exists());
    }
}
