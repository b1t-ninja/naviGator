![ali.svg](ali.svg)
### `cd` but better
#### What is it?
It's basically `cd` but with fuzzy finding and or 'starts with' matching.
So you don't have to type the entire path anymore.

#### How to install it?
To actually be able to navigate to a directory for you, you need to do the following:
- download the repo 
- run `cargo build --release`
- move the binary to a directory of your choice via `mv <somewhere>`
- add the following line to your `.bashrc` or `.zshrc` file: 
    ```bash
  w() {
    local target
    target=$(<path>/<to>/way -c "$PWD" "$@")
    cd "$target"
    }
  ```
- run `source ~/.bashrc` or `source ~/.zshrc` to apply the changes
- enjoy!

#### How to use it?
Now that the binary is registered as a command, you can invoke one of three actions:
- `w <path>` - navigates to the best match of the given path (for example `w u` will navigate to the first matching dir that starts with `u`)
- `w -f <path>` - navigates to the fist best match of the given path (by fuzzy matching)
- `w -i <path>` or `w -f -i <path>`  - enables interactive mode, so if you have multiple matches, you can select the one you want (`-f` enables fuzzy matching)

Oh `<path>` can be any depth, so you can do `<path>/<subpath>/<subsubpath>` and it will still work. Fuzzy or starts with matching will be applied to the subpaths just the same.
