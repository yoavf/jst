Output docs/demos/largest-files.gif

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
Type "mkdir -p /tmp/jst-demo-largest; cd /tmp/jst-demo-largest; mkfile 12m video-cache.bin; mkfile 6m backup.tar; mkfile 2m notes.log; touch small.txt"
Enter
Ctrl+L
Show

Type "jst show the 10 largest files in this folder"
Enter
Sleep 5s
Sleep 2s
