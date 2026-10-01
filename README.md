![nanami.svg](nanami.svg)

### Nanami 🐆

*"I'll take you there. Then I'm clocking out."*

#### Why?

I was a salaryman once. I quit because the work was pointless. I came back
anyway, because it turns out all work is pointless and at least this kind
helps someone. My terms have not changed: I do exactly what is needed, I do it
properly, and I go home at six.

You typing `cd ~/Documents/Misc/Rust/Nanami/src` is overtime. Tabbing through
five levels of completion is overtime. Half remembering a folder name and
`ls`-ing around until you find it is unpaid overtime. I do not approve of any
of it.

So tell me roughly where you want to go. `n do/mi/ru/na/sr` from `~`, or just
`n R/N/sr` from `~/Documents/Misc`, and I take you there. Everything `cd`
understands, I understand too: `..`, `.` and absolute paths (`/usr/lo`,
`~/Doc`). I simply refuse to make you type more than you have to.

#### How do I work?

My technique is the *Ratio*: split the target into parts and strike each one at
its weak point. Your path gets the same treatment.

1. **Split.** I cut the path into its parts: `n ../pro/sr` is "go up", then `pro`, then `sr`.
2. **Strike each part.** `.`, `..` and `/` I take literally, like `cd`. Every
   other part I match against the dirs at that level, by 'starts with'
   (default) or fuzzy (`-f`). Each match becomes the base for the next part.
3. **Rank. Don't dither.** Exact names win, then dirs you have picked before,
   then visible over hidden, then the closest name (the details are
   [below](#which-match-wins)). The best one is where you land.
4. **Clock out.** I print the dir, your shell function `cd`s into it. No match,
   or you cancel: you stay where you are. I do not leave work half done.

When there is a real choice to make, `-i` has me open a picker instead of
guessing. Guessing wrong is how overtime starts.

#### Hiring me

I need a binary and a shell function. Nothing more.

- download the repo
- run `cargo build --release`
- move the binary somewhere sensible: `mv target/release/nanami <somewhere>`
- add this function to your `.bashrc` or `.zshrc`:

```bash
# I have named the function n, feel free to choose your own
n() {
  # help and version print straight to the terminal instead of being cd'd into
  case " $* " in *" -h "*|*" --help "*|*" -V "*|*" --version "*) <path-to-bin> "$@"; return ;; esac
  local target
  target=$(<path-to-bin> "$@") && cd -- "$target"
}
```

- run `source ~/.bashrc` or `source ~/.zshrc`
- go home on time

`n -h` gives you the short briefing with examples. The `&&` makes sure nothing
happens when there is no match or you cancel the selector (I exit non-zero: 1
for no match, 130 for a cancel). The `--` covers dir names that start with a
`-`. Some people name things badly. It is not my place to judge.

#### Giving me work

- `n` - no path, I take you home. Like `cd`. A sensible instinct.
- `n <path>` - I go to the best match (`n u` goes to the best dir that starts with `u`)
- `n -f <path>` - same, fuzzy matching (`n -f dcs` finds `Documents`)
- `n -i <path>` or `n -f -i <path>` - interactive: with more than one match, a small tui lets you pick. With a single match I just go, because asking would be a meeting that could have been an email. Flags can go before or after the path.

The selector opens with the list focused, Vim style. `/` jumps into the fuzzy
search bar on top; type to narrow the list (`nasr` finds `Rust/Nanami/src`),
then `Esc` to go back to the list with the filter kept. The focused box has the
gold border. I match the search against the part of the paths below the dir
they all share, so `go` doesn't hit the `goflink` in every home path.

You can also browse from the list like a file manager. Going into a dir lists
the dir itself (so `Enter` still picks it) and everything in it, hidden dirs
last. Going back lists the dir above, with the one you came from highlighted;
before browsing, back starts from the highlighted dir.

A note on equipment: `Shift-Enter` / `Ctrl-Enter` only work in terminals with
the kitty keyboard protocol (Ghostty, kitty, WezTerm, iTerm2 with CSI u on).
Apple Terminal and IntelliJ send a plain `Enter` for both. Use `Tab` /
`Shift-Tab` there. The tools you are given are rarely the tools you want.

| Key | List (default) | Search bar |
|---|---|---|
| `j` / `k` | down / up | typed into the search |
| `↑` `↓`, `Ctrl-J` `Ctrl-K`, `Ctrl-N` `Ctrl-P` | down / up | down / up |
| `/` | focus the search bar | typed into the search |
| `Backspace`, `Ctrl-U` | – | delete a char, clear the search |
| `Enter` | go there | go there |
| `Shift-Enter`, `Tab`, `→`, `l` | browse into the dir | browse into the dir (`l` is typed) |
| `Ctrl-Enter`, `Shift-Tab`, `←`, `h` | browse one dir up | browse one dir up (`h` is typed) |
| `Esc` | cancel, stay where you are | back to the list |
| `q` | cancel | typed into the search |
| `Ctrl-C` | cancel | cancel |

##### Paths

- **Any depth:** `n R/n/sr` goes to `Rust/Nanami/src`. I match every part on its own.
- **Backwards:** `n ..`, `n ../..`, or mixed: `n ../../pro/sr`. Inside a symlinked dir, `..` goes where `cd ..` would.
- **Pick how far up:** `n -i ..` (or `n -i ../`) lists every parent dir, closest first: from `~/Documents/Misc` that is `~/Documents`, `~`, `/Users`, `/`. `n -i ../..` starts one level higher. Without `-i`, `n ..` is just `cd ..`.
- **Search deeper:** a trailing `/` has me match the last part against every dir up to 3 levels deep, not just the ones right here, and offer everything inside the matches too: top level dirs first, then the next level, shortest path first on each level. From `~`, `n -i do/mi/go/` lists every dir starting with `go` anywhere below `Documents/Misc`, and `n -i ru/` lists `Rust`, `Rust/Nanami`, `Rust/Nanami/src`, … Closer matches come first, hidden dirs last. `.git`, `node_modules` and `target` I only enter when you name them right there (`n -i .git/`); nobody goes in there voluntarily. I stop at 1000 dirs, so `n -i ~/` stays quick. Without `-i`, `n go/` just goes to the best match. If there is only one dir to offer, I go straight there.
- **Absolute:** `n /usr/lo` goes to `/usr/local`, `n ~/Lib/app sup` to `~/Library/Application Support`.
- **Spaces:** quotes are optional, and I match every word on its own: `n app sup`, `n a s` and `n "Application Supp"` all find `Application Support`.

##### Hidden dirs

- I always search hidden dirs; the dot is optional: `n conf` finds `.config`. A visible dir still wins when both match, so `n Doc` goes to `Documents`, not `.docker`.
- `n .` (or `n some/dir/.`) matches all hidden dirs there, so `n -i .` lets you pick one. If there are none, `n .` stays where you are, like `cd .`.
- A `.` anywhere else means "here", like in `cd`: `n ./src` is `src` in the current dir.

##### Which match wins?

I do not flip coins. Ties are broken in this order, every time:

1. An exact name, so `n src` behaves like `cd src` even if `srcold` exists.
2. Dirs you picked in the selector before. Choose `Application Support` once via `n -i app`, and a plain `n app` goes there from then on. I only remember selector picks, in `~/.local/state/nanami/history` (or `$XDG_STATE_HOME/nanami/history`). Delete that file and I forget them. No hard feelings.
3. Visible dirs before hidden ones you matched without typing the dot.
4. The closest name: the shortest one for 'starts with', the best score for fuzzy.
5. Alphabetical, so the same command always lands in the same place. Consistency is a form of respect.

#### Examples

```bash
  n fi
```

I go to the best dir that starts with `fi`.

```bash
  n -i fi
```

I open the small tui and you choose which dir starting with `fi` to go to. For
when you know there are several matches, but you are not paid enough to type
the name any further. Neither am I.

```bash
  n -f fi
```

I go to the best dir that contains `f` followed by `i` (fuzzy).

```bash
  n -f -i fi
```

The tui again, with fuzzy matching.

```bash
  n ../../lib/app sup/goo
```

Up two dirs, then down into `Library/Application Support/Google`. Done.
It's six. I'm going home.
