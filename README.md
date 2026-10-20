# lazykeifu（仮称）

keifuのGitグラフを土台に、lazygitの操作感を目指すRust製TUIクライアントです。**現在はforkの初期化段階で、画面・キー操作はkeifu由来のままです。**

- [当面の開発計画](docs/development-plan.md) / [初期化の検証結果](docs/bootstrap-validation.md)
- [設定](docs/configuration.md) / [TUIデバッグ](docs/debugging.md)
- [上流keifuの操作説明](docs/upstream/README.md)
- [参照用lazygit fork](https://github.com/watabegg/lazygit)

```sh
cargo build --locked
# 対象のGitリポジトリ内で実行
/path/to/lazykeifu/target/debug/lazykeifu
```

設定はOSの設定ディレクトリ内の `lazykeifu/config.toml`（Linuxでは通常 `~/.config/lazykeifu/config.toml`）、ログレベルは `LAZYKEIFU_LOG` を使用します。既存のkeifuの設定は読みません。現在の既定動作では60秒ごとにoriginへfetchします。変更方法は設定ドキュメントを参照してください。

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

[keifu](https://github.com/trasta298/keifu)の履歴とMITライセンスを引き継いでいます。lazygitは操作仕様の参照元で、現段階ではコードを取り込んでいません。名称と配布方針が固まるまでCargo publishと自動リリースを停止しています。
