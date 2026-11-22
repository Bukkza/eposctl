# Game presets

eposctl ships footstep-oriented presets for VALORANT, Delta Force, Call of Duty
and PUBG. These starting points use the published H3PRO Hybrid response and the
Sonar game presets below to refine the earlier community-based curves. The
November 2026 review found no comparable set of controlled recordings from the
current builds that could validate the four curves in a match.

Values are in dB for the nine bands at 62.5, 125, 250, 500 Hz and 1, 2, 4, 8,
16 kHz. All four turn virtual surround and reverb off and leave the microphone
settings alone.

| Preset | 62.5 | 125 | 250 | 500 | 1k | 2k | 4k | 8k | 16k |
|---|---|---|---|---|---|---|---|---|---|
| VALORANT    | -1.5 | 0    | +0.5 | -1.5 | 0    | +2.0 | +1.5 | 0    | 0 |
| Delta Force | -2.0 | +0.5 | +1.0 | -1.0 | +1.0 | +2.5 | +1.0 | -0.5 | 0 |
| Call of Duty | -3.0 | -0.5 | 0    | -0.5 | +0.5 | +2.5 | +1.5 | +0.5 | 0 |
| PUBG        | -3.0 | -1.5 | 0    | 0    | +1.5 | +2.5 | +0.5 | +1.0 | 0 |

## November 2026 review

| Preset | Current evidence | First comparison to run |
|---|---|---|
| VALORANT 13.10 | Riot explains how footstep distance is mixed and supports in-game HRTF or a Windows spatial renderer. The public notes give no basis for a boost at 125 Hz. [Audio Q&A](https://playvalorant.com/en-us/news/game-updates/ask-valorant-6/) · [HRTF and spatial audio](https://playvalorant.com/en-gb/news/game-updates/valorant-patch-notes-8-0/) · [13.10 notes](https://playvalorant.com/en-gb/news/game-updates/valorant-patch-notes-13-10/) | With the same HRTF mode, compare the previous +1 dB at 125 Hz and +3 dB at 4 kHz with the new 0 dB and +1.5 dB during gunfire. |
| Delta Force Aurora | The S12 PC Verity preview changes footsteps, gunfire, explosions and spatial audio on Zenith Port. Its scope is specific to that map. [S12 notes](https://deltaforce.garena.com/en/news/announcement/GWPGVY) | Test Warfare with Verity both ways and test Operations in a separate scene. Check whether the 250 Hz lift helps over vehicles and explosions before using it for both modes. |
| Call of Duty / Warzone | The original preset used Black Ops 6-era references. MW4 Season 1 integrated a new build, and Raven documented quieter teammate steps and Zodiac footstep and occlusion changes. The same preset may behave differently in Warzone and multiplayer. [Warzone Season 1 notes](https://www.callofduty.com/patchnotes/2026/11/call-of-duty-mw4-warzone-season-1-patch-notes) | Prioritize a new recording of the same surfaces and distances in the current mix. Compare the 250 Hz cut and 2–4 kHz lift with a flatter curve. |
| PUBG 44.1 | KRAFTON described material and distance dependence in its footstep rebalance, including reducing high-frequency content with distance. The 44.1 notes provide no new frequency data. [Footstep developer letter](https://steamcommunity.com/games/578080/announcements/detail/2711680989604648300) · [44.1 notes](https://pubg.com/en/news/11338) | Compare the new +1 dB at 8 kHz against the previous +2 dB, on concrete and grass at close and farther distances. Include gunfire in the comparison. |

**Headphone response.** Early tuning used the related H3 Hybrid as a proxy. The
H3PRO Hybrid has its own published measurement. That sample sits a little above
the publisher's target in the bass and low mids, and above the target around
4 kHz. The new curves therefore reduce the 4 kHz lift by 1.5–2 dB and preserve
more of the boost near 2 kHz. The SoundGuys target is a headphone listening
target. The review does not specify the connection or ANC state for that chart,
and fit can change the acoustic response. Treat the adjustments as starting
points for this model. [H3PRO Hybrid measurement](https://www.soundguys.com/epos-h3pro-hybrid-review-62968/)

**Footstep spectra and spatial cues.** Footstep timbre varies with surface,
shoe, movement and the game's mix. A static EQ changes all sounds sharing a
frequency band. Use the same spatial renderer when comparing EQ settings so
that EQ and localization changes can be evaluated independently. The game
presets disable eposctl's virtual surround and reverb. [Footstep synthesis and
listening tests](https://doi.org/10.1016/j.apacoust.2015.05.013) · [FPS
localization study](https://doi.org/10.1109/I3DA48870.2021.9610934)

**GitHub references.** EQ Loadout maintains editable per-game curves and
documents their community provenance. AutoEq builds device-specific correction
from headphone measurements. The two projects address different parts of the
signal chain, and neither validates the curves above against the current game
mixes. [EQ Loadout presets](https://github.com/luanfernandes/eqloadout/blob/main/presets/README.md)
· [AutoEq](https://github.com/jaakkopasanen/AutoEq)

## SteelSeries Sonar comparison

Read the factory **Game** configurations from the installed SteelSeries GG
124.0.0 / Sonar 1.107.0 preset database. The values below are representative
filter settings, before the gains from overlapping filters are combined.
The MW4 preset is distinct from the Warzone preset. [SteelSeries MW4 release notes](https://techblog.steelseries.com/2026/11/03/GG-notes-123.0.0.html)

| Sonar preset | Representative filters | How it informed the H3PRO tuning |
|---|---|---|
| Valorant Pro Preset | 100 Hz high-pass, 250 Hz +6 dB, narrow boosts at 3.14 and 3.51 kHz | Move some of the low boost from 125 to 250 Hz. |
| Delta Force | 80 Hz high-pass, 100 Hz +10 dB, 225 Hz +4 dB, 320 Hz −4 dB | Add 0.5 dB each at 125 and 250 Hz. |
| COD Warzone | 100 Hz high-pass, 125 and 230 Hz +6 dB, 2.89 kHz +6 dB at Q 10 | Set 125 and 250 Hz closer to flat. The previous COD preset cut both by 1.5 dB. |
| COD: Modern Warfare 4 | 60 Hz high-pass, boosts at 125 and 230 Hz, narrow boosts at 2.24 and 2.89 kHz | Compare MW4 multiplayer and Warzone separately when evaluating the shared COD preset. |
| PUBG: BATTLEGROUNDS | 1.2 kHz +5 dB, 3.729 kHz −6 dB at Q 10, 9 kHz +4 dB | Shift the emphasis toward 1–2 kHz and reduce the broad 4 kHz lift. |

Smart Volume and virtual surround were off in all five factory configurations.
The parameter comparison supports testing lower-frequency cues and narrower
filters. A 48 kHz biquad simulation followed by eposctl's ordinary import
still had local errors of 4.2–6.3 dB between 100 Hz and 10 kHz after subtracting
the mean gain offset. A narrow Q 10 filter is better represented in the
parametric editor than by raising or lowering an entire octave-wide control.
These are comparisons of saved settings and modelled responses. In-game
detection scores and the actual Sonar processing curve require separate tests.

## What the EPOS EQ implements

The installed `EAPO.dll` 1.6.1.2 was checked offline. It contains the nine
centres above and two different coefficient tables selected by the sampling
rate. The 81 values in the 48 kHz table match the rounded model in
[`src/peq.rs`](../src/peq.rs) to within `4.9e-7`. The curve between band centres
is modelled. See the [EQ bands in the protocol notes](protocol.md#eq-bands) and
the [cascade graphic EQ paper](https://doi.org/10.1109/LSP.2016.2645280).

| Preset | Modelled maximum boost | Approximate peak frequency |
|---|---:|---:|
| VALORANT | +2.02 dB | 2.09 kHz |
| Delta Force | +2.50 dB | 2.00 kHz |
| Call of Duty | +2.51 dB | 2.04 kHz |
| PUBG | +2.51 dB | 1.95 kHz |
| EPOS factory eSport | +6.08 dB | 15.15 kHz |

These are model estimates relative to flat over 44 Hz to 20 kHz. They help
size digital headroom and do not establish clipping in the driver. The factory
eSport preset also enables virtual surround and about 30% reverb. It changes
more than EQ, so use fixed spatial settings to compare tonal differences.

To validate a change, record the same replay or custom-match paths on the
current build with flat EQ. Keep the region, mode, map, in-game mix, HRTF,
headset connection and ANC fixed. Include footsteps alone, the background
alone, and footsteps during nearby gunfire or explosions. Compare flat, the
current preset and each candidate at matched loudness. Report detection and
direction errors separately, and check peak levels. A waveform extracted from
a game's assets omits the distance, occlusion and mixing that need to be tested.
