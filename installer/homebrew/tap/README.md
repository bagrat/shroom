# Homebrew tap for shroom

Install [shroom](https://shroom.am) via Homebrew:

```bash
brew install --cask bagrat/shroom/shroom
```

shroom keeps itself up to date, so `brew upgrade` leaves it be (add `--greedy` to have
Homebrew update it anyway). It lives in `~/Applications`, where the install line at
[shroom.am](https://shroom.am) puts it too: if you installed it that way, add `--adopt`
to the command above to hand that copy to Homebrew.

To remove it, and its settings with it: `brew uninstall --zap --cask shroom`.

`Casks/shroom.rb` is generated: its source, and the script that fills in each release,
live in [bagrat/shroom](https://github.com/bagrat/shroom/tree/main/installer/homebrew).
Trouble, questions: [issues](https://github.com/bagrat/shroom/issues/new).
