# Markdown Pocket v2.0 — Windows Portable Rust Markdown Editor

[![Build Windows Portable EXE](https://github.com/daichingale/markdown-pocket/actions/workflows/build-windows.yml/badge.svg)](https://github.com/daichingale/markdown-pocket/actions/workflows/build-windows.yml)

学校PC向けの **インストール不要 / PowerShell不要 / 単体EXE** を目標にしたRust製Markdownエディターです。

## v2.0 の機能
- Markdown / TXT を開いて編集
- リアルタイムMarkdownプレビュー
- 複数タブ
- ドラッグ&ドロップでファイルを開く
- 最近使ったファイル（最大12件）
- ライト / ダークモード
- 折り返しON/OFF
- Ctrl+N / Ctrl+O / Ctrl+S
- HTML書き出し
- PDFは HTML書き出し → ブラウザの印刷 → **Microsoft Print to PDF** で作成
- 未保存内容を8秒ごとにリカバリーファイルへ自動保存
- 設定とリカバリーはEXE横の `MarkdownPocketData` に保存する完全ポータブル方式

## 学校PCで使うもの
ビルド後の `markdown-pocket.exe` だけで起動できます。インストーラーはありません。
設定や自動保存を残したい場合は、EXEを置いたフォルダに書き込み権限が必要です。

## GitHub ActionsでEXEを作る
`main` ブランチへ変更を反映すると、GitHub Actionsの `Build Windows Portable EXE` が自動実行されます。手動で実行する場合は、[ワークフロー画面](https://github.com/daichingale/markdown-pocket/actions/workflows/build-windows.yml)を開き、**Run workflow** を選択してください。

### 完成したEXEのダウンロード方法

1. [Build Windows Portable EXE の実行一覧](https://github.com/daichingale/markdown-pocket/actions/workflows/build-windows.yml)を開きます。
2. 緑色のチェックが付いた最新の実行をクリックします。
3. 実行結果ページの一番下にある **Artifacts** までスクロールします。
4. **Markdown-Pocket-Windows-Portable** をクリックしてZIPをダウンロードします。
5. ZIPを展開すると、中に `markdown-pocket.exe` が入っています。

> GitHub ActionsのArtifactsをダウンロードするには、GitHubへのログインが必要です。

## ローカルでビルドする場合
Rustが入ったPCで:

```text
cargo build --release
```

`target/release/markdown-pocket.exe` が完成版です。

## PDFについて
PDF生成エンジンをEXEに内蔵するとサイズと依存が大きくなるため、軽量性を優先して内蔵していません。
「出力 → HTMLとして保存」後、Edge/Chrome等でHTMLを開き、Ctrl+P → Microsoft Print to PDF でPDF化できます。
