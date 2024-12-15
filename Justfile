viewman:
    pandoc man/tomate.1.md -s -t man | /usr/bin/man -l -

buildman:
    pandoc --standalone --to man man/tomate.1.md -o tomate.1
