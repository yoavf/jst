Output docs/demos/remove-ds-store.gif

Require jst

Set Shell "zsh"
Set FontFamily "Monaco"
Set FontSize 18
Set Width 1120
Set Height 720
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
Set CursorBlink false

Hide
Type `unset NO_COLOR; export COLUMNS=80; export PS1=$'\n❯ '`
Enter
Type "mkdir -p /tmp/jst-demo-ds-store/nested; cd /tmp/jst-demo-ds-store; touch .DS_Store nested/.DS_Store"
Enter
Ctrl+L
Show

Type "jst remove all .DS_Store files below the current directory"
Enter
Sleep 7s
Type "n"
Enter
Sleep 2s
