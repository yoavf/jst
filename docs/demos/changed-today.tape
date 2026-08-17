Output docs/demos/changed-today.gif

Require jst

Set Shell "zsh"
Set FontFamily "Monaco"
Set FontSize 18
Set Width 1120
Set Height 560
Set Padding 28
Set Margin 34
Set MarginFill "#f3efe6"
Set BorderRadius 14
Set WindowBar "Colorful"
Set WindowBarSize 38
Set Theme "Catppuccin Mocha"
Set TypingSpeed 34ms
Set Framerate 30
Set PlaybackSpeed 1.15

Hide
Type `unset NO_COLOR; export COLUMNS=80; export PS1=$'\n❯ '`
Enter
Type "mkdir -p /tmp/jst-demo-changed; cd /tmp/jst-demo-changed; touch draft.md deploy.log; touch -t 202501010000 old-notes.txt"
Enter
Ctrl+L
Show

Type "jst find every file changed in the last 24 hours"
Enter
Sleep 5s
Sleep 2s
