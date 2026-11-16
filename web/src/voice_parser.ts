/**
 * 音声認識テキスト（＋自由入力テキスト）からライバーの診断項目を自動抽出する自然言語パーサー
 *
 * 発話例:
 * 「20歳で東京に住んでて、去年10月から始めて50万、今年は250万振り込まれて、経費は30万くらい」
 * 「二十歳、大阪、バイトあり月8万、去年はゼロ、今年180万、経費20万、青色申告してる」
 */

export interface ParsedVoiceInput {
  /** 抽出された年齢（15〜65） */
  age?: number;
  /** 年齢区分ボタン ('u18' | 'student' | 'adult') */
  ageGroup?: 'u18' | 'student' | 'adult';
  /** 都道府県インデックス（0〜47） */
  prefecture?: number;
  /** 都道府県名（表示用） */
  prefectureName?: string;
  /** 2025年（昨年）の振込額（円） */
  income2025?: number;
  /** 2025年（昨年）の経費（円） */
  expense2025?: number;
  /** 2026年（今年）の振込額（円） */
  income2026?: number;
  /** 2026年（今年）の経費（円） */
  expense2026?: number;
  /** 2025年の開始月（1〜12） */
  startMonth2025?: number;
  /** 本業・バイト給与の有無 */
  hasSalary?: boolean;
  /** バイト・給与の年額（円） */
  salaryAnnual?: number;
  /** 会社の社会保険に加入しているか */
  salaryHasShaho?: boolean;
  /** 青色申告（65万控除）の有無 */
  blueReturn?: boolean;
  /** 2025年分を申告済みか */
  filed2025?: boolean;
  /** 一人暮らしか */
  livesAlone?: boolean;
  /** 抽出できた項目のサマリーチップ（UI表示用） */
  detectedTags: string[];
}

/** 47都道府県＋全国平均のエイリアス辞書（インデックスは Rust 側 NHI_TABLE と完全一致） */
const PREF_ALIASES: Array<{ id: number; name: string; keys: string[] }> = [
  { id: 0, name: '北海道', keys: ['北海道', '札幌', 'ほっかいどう', 'さっぽろ'] },
  { id: 1, name: '青森県', keys: ['青森', 'あおもり'] },
  { id: 2, name: '岩手県', keys: ['岩手', '盛岡', 'いわて'] },
  { id: 3, name: '宮城県', keys: ['宮城', '仙台', 'みやぎ', 'せんだい'] },
  { id: 4, name: '秋田県', keys: ['秋田', 'あきた'] },
  { id: 5, name: '山形県', keys: ['山形', 'やまがた'] },
  { id: 6, name: '福島県', keys: ['福島', 'ふくしま'] },
  { id: 7, name: '茨城県', keys: ['茨城', '水戸', 'いばらき'] },
  { id: 8, name: '栃木県', keys: ['栃木', '宇都宮', 'とちぎ'] },
  { id: 9, name: '群馬県', keys: ['群馬', '前橋', '高崎', 'ぐんま'] },
  { id: 10, name: '埼玉県', keys: ['埼玉', 'さいたま', '大宮', '浦和'] },
  { id: 11, name: '千葉県', keys: ['千葉', 'ちば', '船橋', '柏'] },
  { id: 12, name: '東京都', keys: ['東京', 'とうきょう', '都内', '23区', '新宿', '渋谷', '池袋', '八王子'] },
  { id: 13, name: '神奈川県', keys: ['神奈川', '横浜', '川崎', 'かながわ', 'よこはま'] },
  { id: 14, name: '新潟県', keys: ['新潟', 'にいがた'] },
  { id: 15, name: '富山県', keys: ['富山', 'とやま'] },
  { id: 16, name: '石川県', keys: ['石川', '金沢', 'いしかわ', 'かなざわ'] },
  { id: 17, name: '福井県', keys: ['福井', 'ふくい'] },
  { id: 18, name: '山梨県', keys: ['山梨', '甲府', 'やまなし'] },
  { id: 19, name: '長野県', keys: ['長野', 'ながの'] },
  { id: 20, name: '岐阜県', keys: ['岐阜', 'ぎふ'] },
  { id: 21, name: '静岡県', keys: ['静岡', '浜松', 'しずおか'] },
  { id: 22, name: '愛知県', keys: ['愛知', '名古屋', 'あいち', 'なごや'] },
  { id: 23, name: '三重県', keys: ['三重', '津市', '四日市', 'みえ'] },
  { id: 24, name: '滋賀県', keys: ['滋賀', '大津', 'しが'] },
  { id: 25, name: '京都府', keys: ['京都', 'きょうと'] },
  { id: 26, name: '大阪府', keys: ['大阪', 'おおさか', '梅田', '難波', 'なんば'] },
  { id: 27, name: '兵庫県', keys: ['兵庫', '神戸', '西宮', '姫路', 'ひょうご', 'こうべ'] },
  { id: 28, name: '奈良県', keys: ['奈良', 'なら'] },
  { id: 29, name: '和歌山県', keys: ['和歌山', 'わかやま'] },
  { id: 30, name: '鳥取県', keys: ['鳥取', 'とっとり'] },
  { id: 31, name: '島根県', keys: ['島根', '松江', 'しまね'] },
  { id: 32, name: '岡山県', keys: ['岡山', 'おかやま'] },
  { id: 33, name: '広島県', keys: ['広島', 'ひろしま'] },
  { id: 34, name: '山口県', keys: ['山口', 'やまぐち'] },
  { id: 35, name: '徳島県', keys: ['徳島', 'とくしま'] },
  { id: 36, name: '香川県', keys: ['香川', '高松', 'かがわ'] },
  { id: 37, name: '愛媛県', keys: ['愛媛', '松山', 'えひめ'] },
  { id: 38, name: '高知県', keys: ['高知', 'こうち'] },
  { id: 39, name: '福岡県', keys: ['福岡', '博多', '北九州', 'ふくおか', 'はかた'] },
  { id: 40, name: '佐賀県', keys: ['佐賀', 'さが'] },
  { id: 41, name: '長崎県', keys: ['長崎', 'ながさき'] },
  { id: 42, name: '熊本県', keys: ['熊本', 'くまもと'] },
  { id: 43, name: '大分県', keys: ['大分', 'おおいた'] },
  { id: 44, name: '宮崎県', keys: ['宮崎', 'みやざき'] },
  { id: 45, name: '鹿児島県', keys: ['鹿児島', 'かごしま'] },
  { id: 46, name: '沖縄県', keys: ['沖縄', '那覇', 'おきなわ', 'なは'] },
];

/** 全角数字・漢数字を半角アラビア数字に正規化する */
function normalizeJapaneseNumbers(raw: string): string {
  // 全角数字 → 半角
  let s = raw.replace(/[０-９]/g, (c) => String.fromCharCode(c.charCodeAt(0) - 0xfee0));
  s = s.replace(/，/g, '').replace(/,/g, '');

  // 音声認識でよくある表記ゆれ（まんえん → 万円、せんえん → 千円、えん → 円）
  s = s.replace(/まんえん|マンエン/g, '万円');
  s = s.replace(/せんえん|センエン/g, '千円');
  s = s.replace(/(?<=[0-9一二三四五六七八九十百千])まん(?![a-zA-Z])/g, '万');

  // よくある年齢・月の口語表現
  s = s.replace(/二十歳|はたち|ハタチ/g, '20歳');
  s = s.replace(/十八歳/g, '18歳');
  s = s.replace(/十九歳/g, '19歳');

  // 漢数字（百・十・一〜九）の変換（単位が後ろに続く場合も、単独の漢数字も対応）
  const kanjiDigit: Record<string, number> = {
    零: 0, 〇: 0, 一: 1, 二: 2, 三: 3, 四: 4, 五: 5, 六: 6, 七: 7, 八: 8, 九: 9,
  };
  s = s.replace(/[零〇一二三四五六七八九十百千]+(?=[万億歳才カ月ヶ円]|$|[、。！？\s])/g, (match) => {
    let total = 0;
    let current = 0;
    for (const ch of match) {
      if (ch in kanjiDigit) {
        current = kanjiDigit[ch];
      } else if (ch === '千') {
        total += (current || 1) * 1000;
        current = 0;
      } else if (ch === '百') {
        total += (current || 1) * 100;
        current = 0;
      } else if (ch === '十') {
        total += (current || 1) * 10;
        current = 0;
      }
    }
    total += current;
    return total > 0 ? String(total) : match;
  });

  return s;
}

/**
 * 「50万」「250万円」「180万5千円」「300,000円」などを円単位の数値に変換
 */
function parseYenToken(numStr: string, unit: string | undefined): number {
  const n = parseFloat(numStr);
  if (Number.isNaN(n)) return 0;
  if (unit && unit.startsWith('万')) {
    return Math.round(n * 10_000);
  }
  if (unit && unit.startsWith('千')) {
    return Math.round(n * 1_000);
  }
  // 単位なしで 1000 未満の数字が来たら「万円」とみなす（口語で「去年50で今年250」など）
  if (!unit && n > 0 && n < 2000) {
    return Math.round(n * 10_000);
  }
  return Math.round(n);
}

/**
 * 音声・自然言語テキストを解析して構造化データを返す
 */
export function parseVoiceText(rawText: string): ParsedVoiceInput {
  const result: ParsedVoiceInput = { detectedTags: [] };
  if (!rawText || !rawText.trim()) return result;

  const text = normalizeJapaneseNumbers(rawText);

  // 1. 年齢・学生区分の抽出
  const ageMatch = text.match(/(\d{2})\s*(?:歳|才|さい)/);
  if (ageMatch) {
    const age = parseInt(ageMatch[1], 10);
    if (age >= 12 && age <= 75) {
      result.age = age;
      result.ageGroup = age <= 18 ? 'u18' : age <= 22 ? 'student' : 'adult';
      result.detectedTags.push(`🎂 ${age}歳`);
    }
  } else if (/高校生|未成年|18歳以下/.test(text)) {
    result.age = 17;
    result.ageGroup = 'u18';
    result.detectedTags.push('🎂 18歳以下');
  } else if (/大学生|専門学生|学生/.test(text)) {
    result.age = 20;
    result.ageGroup = 'student';
    result.detectedTags.push('🎂 19〜22歳（学生等）');
  } else if (/社会人|大人/.test(text)) {
    result.age = 25;
    result.ageGroup = 'adult';
    result.detectedTags.push('🎂 23歳以上');
  }

  // 2. 都道府県の抽出
  for (const pref of PREF_ALIASES) {
    if (pref.keys.some((k) => text.includes(k))) {
      result.prefecture = pref.id;
      result.prefectureName = pref.name;
      result.detectedTags.push(`📍 ${pref.name}`);
      break;
    }
  }

  // 3. 開始月（去年〇月から）の抽出
  const startMonthMatch = text.match(/(?:去年|昨年|2025年)?[^。、\d]{0,6}?(\d{1,2})\s*月\s*から/);
  if (startMonthMatch) {
    const m = parseInt(startMonthMatch[1], 10);
    if (m >= 1 && m <= 12) {
      result.startMonth2025 = m;
      result.detectedTags.push(`📅 昨年${m}月開始`);
    }
  }

  // 4. 青色申告・白色申告の抽出
  if (/青色|65万控除|開業届出/.test(text)) {
    if (/青色.*(?:ない|してない|出してない|まだ)/.test(text)) {
      result.blueReturn = false;
      result.detectedTags.push('📄 白色・未届');
    } else {
      result.blueReturn = true;
      result.detectedTags.push('📘 青色申告（65万控除）');
    }
  } else if (/白色|何も出してない|開業届出してない/.test(text)) {
    result.blueReturn = false;
    result.detectedTags.push('📄 白色・未届');
  }

  // 5. 昨年分の申告済み・未申告
  if (/未申告|申告してない|まだ出してない/.test(text)) {
    result.filed2025 = false;
    result.detectedTags.push('⚠️ 2025年未申告');
  } else if (/申告済み|申告した|確定申告出した/.test(text)) {
    result.filed2025 = true;
    result.detectedTags.push('✅ 2025年申告済');
  }

  // 6. 一人暮らし or 実家・同居
  if (/一人暮らし|ひとり暮らし|単身/.test(text)) {
    result.livesAlone = true;
    result.detectedTags.push('🏠 一人暮らし');
  } else if (/実家|親と同居|同居/.test(text)) {
    result.livesAlone = false;
    result.detectedTags.push('🏡 実家・親と同居');
  }

  // 7. バイト・会社員給与の有無＆金額
  if (/(?:バイト|アルバイト|会社|本業|給料|給与).*(?:なし|してない|ない|ゼロ|0)|配信のみ|ライバーのみ/.test(text)) {
    result.hasSalary = false;
    result.salaryAnnual = 0;
    result.detectedTags.push('💼 配信のみ（給与なし）');
  } else if (/バイト|アルバイト|会社員|OL|本業|給料|給与|パート/.test(text)) {
    result.hasSalary = true;
    // 「バイト月8万」「給料は年100万」などの抽出
    const salMonthly = text.match(/(?:バイト|給料|給与|パート)[^。、\d]{0,8}?(?:月|つき)\s*(\d+(?:\.\d+)?)\s*(万|万円)?/);
    const salAnnual = text.match(/(?:バイト|給料|給与|パート)[^。、\d]{0,8}?(?:年|年間)?\s*(\d+(?:\.\d+)?)\s*(万|万円)/);
    if (salMonthly) {
      const m = parseYenToken(salMonthly[1], salMonthly[2] || '万');
      result.salaryAnnual = m * 12;
      result.detectedTags.push(`💼 給与 年${Math.round(result.salaryAnnual / 10000)}万（月${salMonthly[1]}万）`);
    } else if (salAnnual) {
      result.salaryAnnual = parseYenToken(salAnnual[1], salAnnual[2]);
      result.detectedTags.push(`💼 給与 年${Math.round(result.salaryAnnual / 10000)}万`);
    } else {
      result.detectedTags.push('💼 バイト・給与あり');
    }
  }

  // 8. 年度別の振込額・経費の抽出
  // 読点「、」や「で、」「て、」などで節に分割し、文脈（去年 / 今年 / 経費）をトラッキングする
  const clauses = text.split(/[、。！!？?\n]|(?:んで|て|で)(?=\s*(?:去年|昨年|今年|経費|バイト|青色))/);
  let currentYearContext: 2025 | 2026 | null = null;

  for (const clause of clauses) {
    if (!clause) continue;

    // 節の中に「去年」「今年」があればコンテキストを更新
    if (/去年|昨年|2025年|令和7年/.test(clause)) {
      currentYearContext = 2025;
    }
    if (/今年|今季|2026年|令和8年|今/.test(clause) && !/去年|昨年/.test(clause)) {
      currentYearContext = 2026;
    }

    // 「去年はゼロ」「去年はなし」対応
    if (currentYearContext === 2025 && /(?:去年|昨年)[^0-9]*(?:ゼロ|なし|ない|0円)/.test(clause)) {
      result.income2025 = 0;
    }

    // 節の中で「去年…〇万」「今年…〇万」「経費…〇万」を個別に探す
    // まず経費の明示マッチ
    const expRegex = /(?:(去年|昨年|2025年|今年|2026年)の?)?\s*経費(?:は|が|で|も)?\s*(?:約|だいたい|ざっくり)?\s*(\d+(?:\.\d+)?)\s*(万|万円|千円|円)?/g;
    let m: RegExpExecArray | null;
    const matchedSpans: Array<[number, number]> = [];

    while ((m = expRegex.exec(clause)) !== null) {
      const yrHint = m[1];
      const val = parseYenToken(m[2], m[3]);
      matchedSpans.push([m.index, m.index + m[0].length]);
      if (yrHint && /去年|昨年|2025/.test(yrHint)) {
        result.expense2025 = val;
      } else if (yrHint && /今年|2026/.test(yrHint)) {
        result.expense2026 = val;
      } else if (currentYearContext === 2025 && result.income2026 === undefined) {
        result.expense2025 = val;
      } else {
        // デフォルトは今年（2026年）の経費としてセット
        result.expense2026 = val;
      }
    }

    // バイト代として既に拾った箇所を除外するため、バイト節はスキップ
    if (/バイト|給料|給与|パート/.test(clause) && !/去年|昨年|今年|振込|配信/.test(clause)) {
      continue;
    }

    // 次に「去年〇万」「今年〇万」など金額表現を抽出（単位なし口語「去年50、今年250」にも対応）
    const moneyRegex = /(\d+(?:\.\d+)?)\s*(万円|万|千円|円)?(?![歳才月年%])/g;
    while ((m = moneyRegex.exec(clause)) !== null) {
      const idx = m.index;
      const numVal = parseFloat(m[1]);
      const unitStr = m[2];
      // 単位がない場合は、文脈キーワード（去年/今年/振込/売上/経費）が近くにある場合のみ金額として扱う
      if (!unitStr) {
        if (!/去年|昨年|今年|今季|振込|振り込|売上|稼|収入|経費/.test(clause)) continue;
        // 2025や2026という西暦そのものや、1〜12の月数は除外
        if (numVal === 2025 || numVal === 2026 || numVal <= 0) continue;
      }
      // 既に経費や青色65万控除としてマッチしていたらスキップ
      if (matchedSpans.some(([s, e]) => idx >= s && idx < e)) continue;
      const around = clause.slice(Math.max(0, idx - 6), idx + m[0].length + 6);
      if (/65万.*控除|青色.*65万/.test(around)) continue;
      if (/バイト|給料|給与/.test(around)) continue;
      if (/\d+\s*(?:歳|才|月|年)/.test(clause.slice(idx, idx + m[0].length + 2))) continue;

      const val = parseYenToken(m[1], unitStr);

      // この金額の直前の文脈を判定
      const before = clause.slice(0, idx);
      if (/経費/.test(before)) {
        if (/去年|昨年|2025/.test(before)) {
          result.expense2025 = val;
        } else {
          result.expense2026 = val;
        }
      } else if (/今年|2026|令和8/.test(before)) {
        result.income2026 = val;
        currentYearContext = 2026;
      } else if (/去年|昨年|2025|令和7/.test(before)) {
        result.income2025 = val;
        currentYearContext = 2025;
      } else if (currentYearContext === 2025 && result.income2025 === undefined) {
        result.income2025 = val;
      } else if (currentYearContext === 2026 && result.income2026 === undefined) {
        result.income2026 = val;
      } else if (result.income2026 === undefined) {
        result.income2026 = val;
      }
    }
  }

  // タグの追加（振込額・経費）
  if (result.income2025 !== undefined) {
    result.detectedTags.push(`💴 2025年振込 ${Math.round(result.income2025 / 10000)}万円`);
  }
  if (result.expense2025 !== undefined) {
    result.detectedTags.push(`🧾 2025年経費 ${Math.round(result.expense2025 / 10000)}万円`);
  }
  if (result.income2026 !== undefined) {
    result.detectedTags.push(`💴 2026年振込 ${Math.round(result.income2026 / 10000)}万円`);
  }
  if (result.expense2026 !== undefined) {
    result.detectedTags.push(`🧾 2026年経費 ${Math.round(result.expense2026 / 10000)}万円`);
  }

  return result;
}
