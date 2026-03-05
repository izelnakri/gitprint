Output demo/demo.gif

Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set Padding 20
Set BorderRadius 12
Set WindowBar Colorful
Set Framerate 30
Set TypingSpeed 40ms

Hide
Type "export PATH=$HOME/.cargo/bin:$PATH && cd /home/izelnakri/Github/gitprint && clear"
Enter
Sleep 1s
Show

Type "# Generate a PDF of this repository"
Sleep 500ms
Enter
Sleep 300ms
Type "gitprint . -o demo/gitprint.pdf"
Sleep 300ms
Enter
Sleep 5s

Type "xdg-open demo/gitprint.pdf"
Sleep 300ms
Enter
Sleep 2s

Type "# Generate a GitHub user activity report"
Sleep 500ms
Enter
Sleep 300ms
Type "gitprint --user izelnakri -o demo/izelnakri.pdf"
Sleep 300ms
Enter
Sleep 5s

Type "xdg-open demo/izelnakri.pdf"
Sleep 300ms
Enter
Sleep 5s

Type "# Preview the repo structure in the terminal"
Sleep 500ms
Enter
Sleep 300ms
Type "gitprint --preview ."
Sleep 300ms
Enter
Sleep 5s

