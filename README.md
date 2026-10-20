# lazykeifu（仮称）

keifuのGitグラフを土台に、lazygitの操作感を目指すRust製TUIクライアントです。グラフを常設し、作業ファイル・ブランチ・必要時の差分を一画面で操作できます。

- [M1実装時の対応範囲](docs/workspace-m1.md)
- [当面の開発計画](docs/development-plan.md) / [初期化の検証結果](docs/bootstrap-validation.md)
- [設定](docs/configuration.md) / [TUIデバッグ](docs/debugging.md)
- [上流keifuの操作説明](docs/upstream/README.md)
- [参照用lazygit fork](https://github.com/watabegg/lazygit)

```sh
cargo build --locked
# 対象のGitリポジトリ内で実行
/path/to/lazykeifu/target/debug/lazykeifu
```

グラフは常に表示し、その下にkeifuのCommit Detail（作者・日時・親・ブランチ・メッセージ）を表示します。`6`で詳細に移動し、`j`／`k`やホイールでスクロールできます。差分を開く間は下段をDiffに切り替え、閉じると詳細に戻ります。Files／Graph／Stashesの`Enter`、または`0`でdiffを開き、`Esc`で閉じます。閉じているdiffはTab巡回の対象外です。狭い端末ではグラフと操作中のペインを上下に並べます（最小20列×10行）。`?`のヘルプは小さめのポップアップで、`j`／`k`やマウスホイールでスクロールし、`Esc`・`?`または外側クリックで閉じます。

作業ペインは画面幅の約4割です。Filesの高さは作業領域の半分で、変更件数やタブを切り替えても動きません。Filesペイン（`2`）ではFiles／Worktrees／Submodules、Branchesペイン（`3`）ではLocal／Remotes／Tagsを`[`／`]`または見出しのクリックで切り替えます。Remotesはリモート追跡ブランチの一覧です。Worktrees／初期化済みSubmodulesの`Enter`でそのリポジトリを開き、Tagsの`Enter`でコミット差分を表示します。

ファイル状態はGitの2列表示です。左がstage済み、右が未stageです。`M`変更（黄）、`A`追加（緑）、`D`削除（赤）、`R`rename（水色）、`??`未追跡（紫）、`UU`などの競合（明るい赤）で種類を区別します。部分stageは`MM`のように両方を表示します。

設定はOSの設定ディレクトリ内の `lazykeifu/config.toml`（Linuxでは通常 `~/.config/lazykeifu/config.toml`）、ログレベルは `LAZYKEIFU_LOG` を使用します。既存のkeifuの設定は読みません。現在の既定動作では60秒ごとにGit設定に従ってfetchします。変更方法は設定ドキュメントを参照してください。

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
# Linuxで実TUIを検証（scriptコマンドが必要）
cargo build --locked
python3 scripts/test-workspace.py
```

[keifu](https://github.com/trasta298/keifu)の履歴とMITライセンスを引き継いでいます。lazygitは操作仕様の参照元で、現段階ではコードを取り込んでいません。名称と配布方針が固まるまでCargo publishと自動リリースを停止しています。

Filesはディレクトリごとに畳めるツリー表示です。フォルダの`Enter`／`Space`で開閉、`←`で親へ移動・折り畳み、`→`で展開・子へ移動します。ペイン移動は`h`／`l`または`Tab`です。画面下には選択中のペインでよく使うキーを表示し、通知が出ていても操作を確認できます。

`c`で中央のコミットダイアログを開きます。stage済みの変更がなければ全stageの確認が出て、`Enter`で未追跡ファイルを含む変更をstageしてダイアログへ進みます。stage済みファイルと選択ファイルのSTAGED差分を並べ、`Tab`／`Shift-Tab`で件名・本文・対象一覧を切り替えます。一覧の`j`／`k`でファイルを確認し、`Space`で対象から外せます（作業ファイルは保持）。件名の`Enter`または`Ctrl-s`でコミットし、本文の`Enter`は改行します。`Ctrl-d/u`で差分をスクロールできます。

`Esc`はコミットの下書きを保持してFilesへ戻り、`c`で再開します（起動中の同じリポジトリ内）。フック・署名の失敗時も入力を保持し、Git出力は`Alt-j/k`やホイールで読めます。入力中にindexやHEADが変わった場合は対象を更新して再確認を求めます。複数行ペーストは件名と本文へ分けて入力し、それだけではコミットしません。

現在の同期操作: `f`でfetch、`p`でpull、`P`でpush。追跡先・`pushRemote`・pull方針などはGit設定に従います。pull／push前には確認画面を表示し、設定不足や競合はGitのエラーを表示します。pullによる自動マージのメッセージはGitの既定値を使用します。

`s`で作業ツリーをstashに保存します（未追跡ファイルを含み、ignoredファイルは除外）。`5`のstash一覧では`Space`でapply、`p`でpop、`d`でdrop、`Enter`で差分へ移動します。apply／popはGitの既定動作と同じく元のstage状態を復元しません。競合したpopではstashを残します。外部でstashを変更したときは`R`で一覧を更新してください。

ファイル一覧の`d`で未stageの変更を確認後に破棄します（indexは保持、未追跡ファイルは削除）。差分ペインでは`[`／`]`でhunkを選び、`Space`でstage／unstageします。hunk操作は変更済みのUTF-8通常ファイルに対応し、新規・削除・rename・モード変更・バイナリはファイル単位で操作します。表示後に差分が変わった場合は`R`で再確認してください。

Branches（`3`）またはGraph（`4`）でブランチを選び、`M`で現在のブランチへmerge、`r`で現在のブランチを選択先へrebaseします。確認画面に方向と対象を表示し、Git本体のエディタ・フック・署名を使って実行します。未コミット変更の自動stashは行いません。競合時はファイルを解消して`C`で続行、`A`で中止できます。

作業ファイルの`e`でGitのエディタ設定（`GIT_EDITOR`／`core.editor`など）を使って編集し、終了後に再読込します。GUIエディタは`code --wait`のように終了を待つ設定が必要です。競合ファイルの`Space`は確認後に現在の内容を解消済みとしてstageします（競合マーカーの意味判断は行いません）。merge／rebase中は`C`で続行、`A`で中止を確認後に実行できます。続行時はGitのエディタ・フック・署名にも端末を渡します。外部エディタの起動には`sh`を使います（WindowsではGit付属の`sh`をPATHに追加する必要があります）。

Graph（`4`）でコミットを選び、`y`でcherry-pick、`V`でrevertします。対象コミットと現在のブランチを確認して実行し、競合時は`C`で続行、`A`で中止できます。マージコミットの基準親指定には未対応です。

履歴は最初に500件を表示し、Graphの`+`で500件ずつ追加読み込みします。選択位置と読み込み範囲は更新後も維持され、フッターで件数と残りの有無を確認できます。`G`は読み込み済み範囲の末尾へ移動します。
