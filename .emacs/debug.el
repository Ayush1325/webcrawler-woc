;;; Dap Mode Config

(dap-register-debug-template
 "Webcrawler GDB Run Configuration"
 (list :type "gdb"
       :request "launch"
       :name "Webcrawler Debug"
       :gdbpath "rust-gdb"
       :program "${workspaceRoot}/target/debug/webcrawler"
       :args ["https://crawler-test.com/" "-d" "1"]
       :stopOnEntry -1
       :cwd "${workspaceFolder}"
       :sourceMap (list "/rusrc/*" "${env:HOME}/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust")))

;;; debug.el ends here
