# 初期化の検証

> 仮称 `lazykeifu` での初期検証記録です。現在の名称は **pedigit** です。

検証環境: Linux、rustc 1.96.1、cargo 1.96.1。

| 確認 | 結果 |
|---|---|
| 上流keifu `015f0b2` の `cargo test --locked` | 113件成功 |
| 名前変更後の `cargo test --locked` | 113件成功 |
| `cargo fmt --check` | 成功 |
| `cargo clippy --all-targets --locked -- -D warnings` | 成功 |
| `cargo run --locked -- --help` / `--version` | `lazykeifu 0.1.0-dev` として起動 |
| 一時リポジトリでのTUI確認 | グラフ描画、TabでDetailへ移動、110×30・54×20の描画、正常終了を確認 |
| 独立した設定ディレクトリ | `XDG_CONFIG_HOME/lazykeifu/config.toml` の設定が反映 |

TUIは既存debug serverへ入力し、stateと画面dumpを検査しました。新しいペイン構成やlazygit互換操作はまだ実装していません。macOS・Windowsでのローカル実行は未検証です。
