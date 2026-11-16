//! # rust_tax_engine
//!
//! TikTokライバー向け「税金・国保・親の扶養」一撃診断の計算エンジン。
//! すべての計算はブラウザ内（WebAssembly）で完結し、外部通信は一切行わない。
//!
//! JS からは JSON 文字列で入出力する（BigInt 変換などの罠を避けるため）。
//!
//! ```js
//! import init, { diagnose_json, prefectures_json } from "./pkg/rust_tax_engine.js";
//! await init();
//! const result = JSON.parse(diagnose_json(JSON.stringify(input)));
//! ```

pub mod dependent;
pub mod diagnose;
pub mod income_tax;
pub mod model;
pub mod nhi;
pub mod pension;
pub mod prefectures;
pub mod resident_tax;
pub mod util;

use serde::Serialize;
use wasm_bindgen::prelude::*;

/// エンジンのバージョン（税制年度＋リビジョン）
pub const ENGINE_VERSION: &str = "2025-2026.r1";

/// 診断を実行する。入力・出力ともに JSON 文字列。
/// 入力の形式は [`model::DiagnoseInput`]、出力は [`model::DiagnoseOutput`]。
/// 入力が壊れている場合は `{"error": "..."}` を返す。
#[wasm_bindgen]
pub fn diagnose_json(input_json: &str) -> String {
    match serde_json::from_str::<model::DiagnoseInput>(input_json) {
        Ok(input) => {
            let out = diagnose::diagnose(&input);
            serde_json::to_string(&out).unwrap_or_else(|e| error_json(&e.to_string()))
        }
        Err(e) => error_json(&format!("入力データの読み取りに失敗しました: {e}")),
    }
}

#[derive(Serialize)]
struct PrefectureOut {
    id: usize,
    name: &'static str,
    city: &'static str,
}

/// 都道府県リスト（47都道府県＋全国平均の48件）を JSON 配列で返す。
#[wasm_bindgen]
pub fn prefectures_json() -> String {
    let list: Vec<PrefectureOut> = prefectures::NHI_TABLE
        .iter()
        .enumerate()
        .map(|(id, p)| PrefectureOut {
            id,
            name: p.name,
            city: p.city,
        })
        .collect();
    serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string())
}

/// エンジンのバージョン文字列
#[wasm_bindgen]
pub fn engine_version() -> String {
    ENGINE_VERSION.to_string()
}

fn error_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_works() {
        let out = diagnose_json(r#"{"age":20,"prefecture":12,"income2025":500000,"income2026":2500000,"expense2026":300000,"startMonth2025":10}"#);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_none(), "{out}");
        assert_eq!(v["years"].as_array().unwrap().len(), 2);
        assert_eq!(v["prefectureName"], "東京都");
    }

    #[test]
    fn broken_json_returns_error() {
        let out = diagnose_json("{not json");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v.get("error").is_some());
    }

    #[test]
    fn prefecture_list_has_48_items() {
        let v: serde_json::Value = serde_json::from_str(&prefectures_json()).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 48);
    }
}
