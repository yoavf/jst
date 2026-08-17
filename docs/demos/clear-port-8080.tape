Output docs/demos/clear-port-8080.gif

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

Hide
Type `unset NO_COLOR; export COLUMNS=80; export PS1=$'\n❯ '`
Enter
Type "python3 -m http.server 8080 >/dev/null 2>&1 &"
Enter
Sleep 1s
Ctrl+L
Show

Type "jst stop whatever is using port 8080"
Enter
Sleep 7s
Type "y"
Enter
Sleep 2s
Type "lsof -nP -iTCP:8080 -sTCP:LISTEN || echo 'port 8080 is free'"
Enter
Sleep 2s
