# TikTokライバー向け「税金・国保・親の扶養」一撃診断 Webアプリ (`Rust-TAX`)

PCを持たずスマホ操作のみ・税金知識ゼロ・数字のポチポチ手打ちを嫌がる若手ライバー向けに、**「マイクで喋るかスライダーを動かすだけ」**で2025年（昨年）分・2026年（今年）分の税金・国保・年金・親の扶養判定を一撃診断できる完全クライアントサイドWebアプリです。

---

## 📁 ディレクトリ構成

```text
Rust-TAX/
├── engine/                     # 🦀 Rust + WebAssembly 計算エンジンクレート
│   ├── Cargo.toml              # wasm-bindgen / serde 設定
│   └── src/
│       ├── lib.rs              # Wasm エクスポート (`diagnose_json`, `prefectures_json`)
│       ├── model.rs            # 入出力データ構造 (`DiagnoseInput`, `DiagnoseOutput` 等)
│       ├── income_tax.rs       # 所得税＋復興特別所得税（2025年基礎控除58〜95万 / 2026年最大104万）
│       ├── resident_tax.rs     # 住民税（基礎控除43万・非課税ライン45万・所得割10%＋均等割5,000円）
│       ├── dependent.rs        # 親の扶養判定（税法58万/62万・特定親族123万、社会保険130万/150万）
│       ├── prefectures.rs      # 47都道府県＋全国平均（計48パターン）国民健康保険料率 const 配列
│       ├── nhi.rs              # 国民健康保険料計算＋7割・5割・2割軽減ロジック
│       ├── pension.rs          # 国民年金（20歳以上・学生納付特例・全額免除案内）
│       ├── diagnose.rs         # 2025/2026統合計算・期限後申告アラート・アクションリスト生成
│       └── util.rs             # 端数処理・金額フォーマット
├── web/                        # 📱 モバイルファースト軽量フロントエンド（静的ホスティング対応）
│   ├── index.html              # スマホ1画面完結UI＆スクショ用診断結果カード
│   ├── styles.css              # TikTokライクなダーク＆ネオン配色・レスポンシブCSS
│   ├── src/
│   │   ├── main.ts             # UI制御・Web Speech API音声認識・スライダー双方向バインディング
│   │   ├── voice_parser.ts     # 音声テキスト自然言語パーサー（年齢・都道府県・年度別金額・経費抽出）
│   │   └── wasm_bridge.ts      # Wasm ローダー＆Rust完全互換ローカル実行ブリッジ
│   └── dist/                   # ブラウザ実行用コンパイル済み ES Modules (`npm run build` で生成)
├── scripts/
│   └── build.mjs               # 外部パッケージ不要の TypeScript → ES Modules & Wasm ビルドスクリプト
├── package.json
└── tsconfig.json
```

---

## 🚀 起動・ビルド方法

### 1. すぐにブラウザで動かす（Rust未インストール環境でも即動作）
`web/dist/` にトランスパイル済みのモジュールが含まれているため、そのままローカルサーバーで開けます。

```bash
# Node.js でトランスパイル（コード編集時）
npm run build

# 静的サーバーで起動（例: python3 または npx serve）
python3 -m http.server 8080 --directory web
# → ブラウザで http://localhost:8080 を開く
```

### 2. Rust + `wasm-pack` で `.wasm` バイナリをビルドする
環境に Rust (`rustup`) と `wasm-pack` が入っている場合は、以下のコマンドで `engine/` クレートが `web/dist/pkg/` に WebAssembly としてビルドされ、フロントエンドが自動的に `.wasm` バイナリをロードします。

```bash
# ユニットテストの実行
cd engine && cargo test

# Wasm ビルド＋フロントエンドビルド
npm run build:wasm
```
