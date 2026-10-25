# Ember

Your Deadlock status in Discord, worded your way.

Ember sits in the tray and tells Discord what you're doing in Deadlock: in the Hideout,
finding a match, in a match as your hero, the result once it's over. Every game state gets
its own two lines, picture and timer, and the window shows a live copy of the card your
friends will see.

## Features

- **A status per game state:** starting, Hideout, finding a match, match found, match
  starting, in a match, post-game, playing with bots, spectating.
- **Your wording, with variables:** `{hero}`, `{rank}`, `{kda}`, `{souls}`, `{result}`,
  `{mode}`, `{duration}` and more.
- **Fallbacks with `||`:** `{result} as {hero} || Match over` shows the second part until
  the results are in.
- **Pictures:** hero icon, hero portrait, rank badge or the Deadlock icon, plus a small badge.
- **Timers:** time in this state, match time or time in the queue.
- **Post-game results** (won or lost, K/D/A, souls, mode) from [deadlock-api.com](https://deadlock-api.com).
- **Up to two buttons** under the status, e.g. your Steam profile.
- **English and Russian** interface.

## How it works

Ember reads the game's own console log (`game/citadel/console.log`, written with the
`-condebug` launch option). It doesn't touch the game: no memory reading, no injection, so
it survives patches and stays away from the anti-cheat. From the log it learns the map, the
queue, the match id, the match phases and your hero.

Heroes, ranks and match results come from the public [deadlock-api.com](https://deadlock-api.com)
API. The status goes to the Discord app on your PC over its local RPC pipe.

### Optional: live numbers from memory

The log doesn't know what happens inside a match. For players who accept the risk, Ember
can read the game's memory to show live K/D/A, souls, level, mode, Street Brawl round and
score, and to tell when you're spectating. It's **off by default** and asks for explicit
confirmation.

- It only reads: no writes, no injected code, no hooks. The reading is done by the
  open-source [deadlock-rs](https://github.com/deadlock-plus/deadlock-rs) library, which
  takes field offsets from the game's own Source 2 schema.
- Valve hasn't approved tools like this. The risk to your account is small but not zero.
- If a patch makes the numbers read wrong, Ember notices and stops using them.

## Setup

1. **Create a Discord application.** Open the
   [Discord Developer Portal](https://discord.com/developers/applications), click
   **New Application** and name it **Deadlock**: that name becomes "Playing Deadlock".
   Copy the **Application ID** into Ember → Setup.
2. **Add `-condebug`** in Steam → Deadlock → Properties → Launch Options.
3. **Optional, for a sharp icon:** Ember → Setup → *Save the icon*, then upload
   `deadlock.png` to your app under **Rich Presence → Art Assets** with the name
   `deadlock`, and turn on *I uploaded the icon named deadlock*.

Ember finds Deadlock through your Steam libraries; you can also set the game folder by hand.

## Building

You need [Node.js](https://nodejs.org) 22+ and [Rust](https://rustup.rs) (stable).

```bash
npm install
npm run tauri dev      # run with hot reload
npm run tauri build    # Windows installer in src-tauri/target/release/bundle
```

Tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Pushing a `v*` tag builds the installer on GitHub Actions and attaches it to a draft release.

## Project layout

| Path | What it is |
|---|---|
| `src-tauri/src/log.rs` | Turns console.log lines into the game state |
| `src-tauri/src/presence.rs` | Templates, variables and the Discord activity |
| `src-tauri/src/discord.rs` | Discord's local RPC pipe |
| `src-tauri/src/api.rs` | deadlock-api.com: heroes, ranks, match results |
| `src-tauri/src/memory.rs` | The optional memory reading |
| `src-tauri/src/steam.rs` | Finds the game, the account and the running process |
| `src/` | The window: React + TypeScript |

## Privacy

Ember sends your Steam account id to deadlock-api.com to fetch your rank and match results.
Nothing else leaves your PC; the status goes to the Discord app running locally.

## Credits

- [deadlock-api.com](https://deadlock-api.com) for hero, rank and match data.
- [deadlock-rs](https://github.com/deadlock-plus/deadlock-rs) (MIT OR Apache-2.0) for the
  optional memory reading.

Ember is a fan project and is not affiliated with or endorsed by Valve. Deadlock and its
icon are trademarks of Valve Corporation.

## License

[MIT](LICENSE)
