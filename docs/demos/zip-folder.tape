Output docs/demos/zip-folder.gif

Require jst

Set Shell "zsh"
Set FontFamily "Monaco"
Set FontSize 18
Set Width 1120
Set Height 620
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
Type "mkdir -p /tmp/jst-demo-zip/folder/node_modules /tmp/jst-demo-zip/folder/src; cd /tmp/jst-demo-zip; mkfile 1m folder/src/app.js; mkfile 2m folder/node_modules/bundle.js"
Enter
Ctrl+L
Show

Type "jst zip this folder without node_modules"
Enter
Sleep 5s
Sleep 2s
Type "unzip -l folder.zip"
Enter
Sleep 2s
