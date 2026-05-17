zsh -lc ps -axo pid,ppid,rss,etime,command | rg 'cass index|cass doctor|cass watchdog'
