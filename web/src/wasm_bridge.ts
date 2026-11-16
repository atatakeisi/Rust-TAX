/**
 * Rust (Wasm) 計算エンジンとのブリッジ＋プリコンパイル済みフォールバック
 *
 * 1. `wasm-pack build --target web` で生成された `./pkg/rust_tax_engine.js` が存在する場合は
 *    本物の Rust Wasm バイナリをロードして実行する。
 * 2. 初回クローン直後など Wasm ビルド前の環境でもそのままブラウザで動作・検証できるよう、
 *    Rust 側（`engine/src/*.rs`）と1円単位まで完全同一ロジックのローカル実行エンジンも内蔵。
 */

export interface DiagnoseInput {
  age: number;
  prefecture: number;
  income2025: number;
  expense2025: number;
  income2026: number;
  expense2026: number;
  hasSalary: boolean;
  salaryAnnual: number;
  salaryHasShaho: boolean;
  blueReturn: boolean;
  filed2025: boolean;
  livesAlone: boolean;
  startMonth2025: number;
  daysSince2025Deadline: number;
}

export type Status = 'na' | 'safe' | 'caution' | 'out';

export interface DependentJudge {
  status: Status;
  label: string;
  value: number;
  line: number;
  message: string;
  parentTaxIncrease: number;
}

export interface YearResult {
  year: number;
  age: number;
  gross: number;
  expense: number;
  blueDeduction: number;
  businessIncome: number;
  salaryIncome: number;
  totalIncome: number;
  basicDeduction: number;
  socialDeduction: number;
  taxableIncome: number;
  incomeTax: number;
  residentTax: number;
  residentTaxLabel: string;
  nhi: number;
  nhiLabel: string;
  nhiReduction: number;
  pension: number;
  total: number;
  taxDependent: DependentJudge;
  insuranceDependent: DependentJudge;
  filingRequired: boolean;
  residentFilingRequired: boolean;
}

export interface LateFiling {
  tax: number;
  penalty: number;
  delinquency: number;
  message: string;
}

export interface Action {
  priority: Status;
  icon: string;
  title: string;
  detail: string;
  when: string;
}

export interface DiagnoseOutput {
  engineVersion: string;
  prefectureName: string;
  prefectureCity: string;
  years: [YearResult, YearResult];
  lateFiling: LateFiling | null;
  lockTotal: number;
  freeMoney: number;
  setAsidePercent: number;
  overall: Status;
  headline: string;
  actions: Action[];
  notes: string[];
}

export interface PrefectureItem {
  id: number;
  name: string;
  city: string;
}

interface NhiRate {
  name: string;
  city: string;
  incomeRateBp: number;
  perCapita: number;
  perHousehold: number;
}

/** Rust `engine/src/prefectures.rs` の `NHI_TABLE` と完全同一の48件テーブル */
export const NHI_TABLE: readonly NhiRate[] = [
  { name: '北海道', city: '札幌市', incomeRateBp: 1100, perCapita: 37000, perHousehold: 33000 },
  { name: '青森県', city: '青森市', incomeRateBp: 1150, perCapita: 40000, perHousehold: 33000 },
  { name: '岩手県', city: '盛岡市', incomeRateBp: 1000, perCapita: 38000, perHousehold: 30000 },
  { name: '宮城県', city: '仙台市', incomeRateBp: 960, perCapita: 38000, perHousehold: 26000 },
  { name: '秋田県', city: '秋田市', incomeRateBp: 1020, perCapita: 37000, perHousehold: 30000 },
  { name: '山形県', city: '山形市', incomeRateBp: 1050, perCapita: 40000, perHousehold: 30000 },
  { name: '福島県', city: '福島市', incomeRateBp: 1020, perCapita: 39000, perHousehold: 28000 },
  { name: '茨城県', city: '水戸市', incomeRateBp: 960, perCapita: 38000, perHousehold: 28000 },
  { name: '栃木県', city: '宇都宮市', incomeRateBp: 1000, perCapita: 40000, perHousehold: 30000 },
  { name: '群馬県', city: '前橋市', incomeRateBp: 950, perCapita: 37000, perHousehold: 27000 },
  { name: '埼玉県', city: 'さいたま市', incomeRateBp: 920, perCapita: 42000, perHousehold: 0 },
  { name: '千葉県', city: '千葉市', incomeRateBp: 940, perCapita: 38000, perHousehold: 22000 },
  { name: '東京都', city: '特別区（23区）', incomeRateBp: 1031, perCapita: 65200, perHousehold: 0 },
  { name: '神奈川県', city: '横浜市', incomeRateBp: 960, perCapita: 49000, perHousehold: 0 },
  { name: '新潟県', city: '新潟市', incomeRateBp: 980, perCapita: 38000, perHousehold: 27000 },
  { name: '富山県', city: '富山市', incomeRateBp: 990, perCapita: 40000, perHousehold: 28000 },
  { name: '石川県', city: '金沢市', incomeRateBp: 1050, perCapita: 42000, perHousehold: 30000 },
  { name: '福井県', city: '福井市', incomeRateBp: 980, perCapita: 39000, perHousehold: 28000 },
  { name: '山梨県', city: '甲府市', incomeRateBp: 960, perCapita: 38000, perHousehold: 27000 },
  { name: '長野県', city: '長野市', incomeRateBp: 950, perCapita: 37000, perHousehold: 26000 },
  { name: '岐阜県', city: '岐阜市', incomeRateBp: 1020, perCapita: 40000, perHousehold: 29000 },
  { name: '静岡県', city: '静岡市', incomeRateBp: 980, perCapita: 40000, perHousehold: 24000 },
  { name: '愛知県', city: '名古屋市', incomeRateBp: 990, perCapita: 52000, perHousehold: 0 },
  { name: '三重県', city: '津市', incomeRateBp: 1020, perCapita: 40000, perHousehold: 30000 },
  { name: '滋賀県', city: '大津市', incomeRateBp: 950, perCapita: 40000, perHousehold: 28000 },
  { name: '京都府', city: '京都市', incomeRateBp: 1060, perCapita: 41000, perHousehold: 27000 },
  { name: '大阪府', city: '大阪市（府内統一）', incomeRateBp: 1150, perCapita: 43000, perHousehold: 40000 },
  { name: '兵庫県', city: '神戸市', incomeRateBp: 1060, perCapita: 44000, perHousehold: 30000 },
  { name: '奈良県', city: '奈良市（県内統一）', incomeRateBp: 1070, perCapita: 44000, perHousehold: 30000 },
  { name: '和歌山県', city: '和歌山市', incomeRateBp: 1080, perCapita: 40000, perHousehold: 31000 },
  { name: '鳥取県', city: '鳥取市', incomeRateBp: 1060, perCapita: 39000, perHousehold: 28000 },
  { name: '島根県', city: '松江市', incomeRateBp: 1050, perCapita: 40000, perHousehold: 30000 },
  { name: '岡山県', city: '岡山市', incomeRateBp: 1060, perCapita: 40000, perHousehold: 30000 },
  { name: '広島県', city: '広島市（県内統一）', incomeRateBp: 1000, perCapita: 40000, perHousehold: 29000 },
  { name: '山口県', city: '山口市', incomeRateBp: 1030, perCapita: 39000, perHousehold: 28000 },
  { name: '徳島県', city: '徳島市', incomeRateBp: 1150, perCapita: 40000, perHousehold: 33000 },
  { name: '香川県', city: '高松市', incomeRateBp: 1060, perCapita: 40000, perHousehold: 30000 },
  { name: '愛媛県', city: '松山市', incomeRateBp: 1050, perCapita: 39000, perHousehold: 29000 },
  { name: '高知県', city: '高知市', incomeRateBp: 1060, perCapita: 39000, perHousehold: 29000 },
  { name: '福岡県', city: '福岡市', incomeRateBp: 1050, perCapita: 38000, perHousehold: 30000 },
  { name: '佐賀県', city: '佐賀市', incomeRateBp: 1180, perCapita: 42000, perHousehold: 34000 },
  { name: '長崎県', city: '長崎市', incomeRateBp: 1080, perCapita: 40000, perHousehold: 31000 },
  { name: '熊本県', city: '熊本市', incomeRateBp: 1030, perCapita: 40000, perHousehold: 30000 },
  { name: '大分県', city: '大分市', incomeRateBp: 1070, perCapita: 40000, perHousehold: 30000 },
  { name: '宮崎県', city: '宮崎市', incomeRateBp: 1080, perCapita: 40000, perHousehold: 30000 },
  { name: '鹿児島県', city: '鹿児島市', incomeRateBp: 1060, perCapita: 40000, perHousehold: 28000 },
  { name: '沖縄県', city: '那覇市', incomeRateBp: 1040, perCapita: 35000, perHousehold: 26000 },
  { name: 'わからない（全国平均）', city: '全国平均の目安', incomeRateBp: 1040, perCapita: 40000, perHousehold: 27000 },
];

const MAN = 10_000;
const KODOMO_RATE_BP = 27;
const KODOMO_PER_CAPITA = 1_900;
const NHI_CAP = 960_000;

function floorTo(v: number, unit: number): number {
  if (v <= 0 || unit <= 0) return 0;
  return Math.floor(v / unit) * unit;
}

function fmtYen(v: number): string {
  const n = Math.max(0, Math.round(v));
  if (n >= MAN) {
    const tenths = Math.floor((n + 500) / 1000);
    const whole = Math.floor(tenths / 10);
    const frac = tenths % 10;
    return frac === 0
      ? `${whole.toLocaleString('ja-JP')}万円`
      : `${whole.toLocaleString('ja-JP')}.${frac}万円`;
  }
  return `${n.toLocaleString('ja-JP')}円`;
}

function basicDeduction(year: number, totalIncome: number): number {
  if (year >= 2026) {
    if (totalIncome <= 489 * MAN) return 104 * MAN;
    if (totalIncome <= 655 * MAN) return 67 * MAN;
    if (totalIncome <= 2350 * MAN) return 62 * MAN;
    if (totalIncome <= 2400 * MAN) return 48 * MAN;
    if (totalIncome <= 2450 * MAN) return 32 * MAN;
    if (totalIncome <= 2500 * MAN) return 16 * MAN;
    return 0;
  } else {
    if (totalIncome <= 132 * MAN) return 95 * MAN;
    if (totalIncome <= 336 * MAN) return 88 * MAN;
    if (totalIncome <= 489 * MAN) return 68 * MAN;
    if (totalIncome <= 655 * MAN) return 63 * MAN;
    if (totalIncome <= 2350 * MAN) return 58 * MAN;
    if (totalIncome <= 2400 * MAN) return 48 * MAN;
    if (totalIncome <= 2450 * MAN) return 32 * MAN;
    if (totalIncome <= 2500 * MAN) return 16 * MAN;
    return 0;
  }
}

function salaryDeduction(salary: number, minDed: number): number {
  const s = Math.max(0, salary);
  let f = 0;
  if (s <= 190 * MAN) f = 0;
  else if (s <= 360 * MAN) f = Math.floor((s * 30) / 100) + 8 * MAN;
  else if (s <= 660 * MAN) f = Math.floor((s * 20) / 100) + 44 * MAN;
  else if (s <= 850 * MAN) f = Math.floor((s * 10) / 100) + 110 * MAN;
  else f = 195 * MAN;
  return Math.min(s, Math.max(f, minDed));
}

function salaryIncomeIt(year: number, salary: number): number {
  const min = year >= 2026 ? 74 * MAN : 65 * MAN;
  return Math.max(0, salary - salaryDeduction(salary, min));
}

function salaryIncomeRt(year: number, salary: number): number {
  const min = year >= 2026 ? 69 * MAN : 65 * MAN;
  return Math.max(0, salary - salaryDeduction(salary, min));
}

function taxOnTaxableIncome(taxable: number): number {
  const t = floorTo(taxable, 1000);
  if (t <= 0) return 0;
  const brackets: Array<[number, number, number]> = [
    [1_950_000, 5, 0],
    [3_300_000, 10, 97_500],
    [6_950_000, 20, 427_500],
    [9_000_000, 23, 636_000],
    [18_000_000, 33, 1_536_000],
    [40_000_000, 40, 2_796_000],
    [Number.MAX_SAFE_INTEGER, 45, 4_796_000],
  ];
  const [, rate, ded] = brackets.find(([u]) => t <= u) ?? [Number.MAX_SAFE_INTEGER, 45, 4_796_000];
  const base = Math.floor((t * rate) / 100) - ded;
  const withRecon = Math.floor((base * 1021) / 1000);
  return floorTo(withRecon, 100);
}

function calcResidentTax(totalIncome: number, socialDed: number, ageAtJan1: number): number {
  if (totalIncome <= 45 * MAN) return 0;
  if (ageAtJan1 < 18 && totalIncome <= 135 * MAN) return 0;
  const taxable = floorTo(totalIncome - 43 * MAN - Math.max(0, socialDed), 1000);
  let incomeLevy = 0;
  if (taxable > 0) {
    const gap = 5 * MAN;
    const adjust =
      taxable <= 200 * MAN
        ? Math.floor((Math.min(taxable, gap) * 5) / 100)
        : Math.max(2500, Math.floor(((gap - (taxable - 200 * MAN)) * 5) / 100));
    incomeLevy = floorTo(Math.floor((taxable * 10) / 100) - adjust, 100);
  }
  return incomeLevy + 5000;
}

function calcNhi(
  prefIndex: number,
  totalIncome: number,
  age: number,
  livesAlone: boolean,
  hasSalary: boolean
): { premium: number; reduction: number } {
  const rate = NHI_TABLE[prefIndex] ?? NHI_TABLE[47];
  const base = Math.max(0, totalIncome - 43 * MAN);
  let reduction = 0;
  if (livesAlone) {
    const extra = 10 * MAN * (Math.max(1, hasSalary ? 1 : 0) - 1);
    const inc = Math.max(0, totalIncome);
    if (inc <= 43 * MAN + extra) reduction = 7;
    else if (inc <= 43 * MAN + 31 * MAN + extra) reduction = 5;
    else if (inc <= 43 * MAN + 57 * MAN + extra) reduction = 2;
  }
  const keep = (amt: number) => Math.floor((amt * (10 - reduction)) / 10);
  const incomeLevy = Math.floor((base * (rate.incomeRateBp + KODOMO_RATE_BP)) / 10000);
  const kodomoCapita = age >= 18 ? KODOMO_PER_CAPITA : 0;
  const flat = keep(rate.perCapita) + keep(rate.perHousehold) + keep(kodomoCapita);
  const premium = floorTo(Math.min(NHI_CAP, incomeLevy + flat), 100);
  return { premium, reduction };
}

function judgeTaxDependent(year: number, age: number, totalIncome: number): DependentJudge {
  const line = year >= 2026 ? 62 * MAN : 58 * MAN;
  const value = Math.max(0, totalIncome);
  if (age < 16) {
    return {
      status: 'na',
      label: '対象外',
      value,
      line,
      message: '16歳未満はもともと親の扶養控除がない年齢なので、親の税金への影響はありません。',
      parentTaxIncrease: 0,
    };
  }
  const specific = age >= 19 && age <= 22;
  const full: [number, number] = specific ? [63 * MAN, 45 * MAN] : [38 * MAN, 33 * MAN];
  let actual: [number, number] = [0, 0];
  if (value <= line) {
    actual = full;
  } else if (specific && value <= 123 * MAN) {
    if (value <= 85 * MAN) actual = [63 * MAN, 45 * MAN];
    else if (value <= 90 * MAN) actual = [61 * MAN, 45 * MAN];
    else if (value <= 95 * MAN) actual = [51 * MAN, 45 * MAN];
    else if (value <= 100 * MAN) actual = [41 * MAN, 41 * MAN];
    else if (value <= 105 * MAN) actual = [31 * MAN, 31 * MAN];
    else if (value <= 110 * MAN) actual = [21 * MAN, 21 * MAN];
    else if (value <= 115 * MAN) actual = [11 * MAN, 11 * MAN];
    else if (value <= 120 * MAN) actual = [6 * MAN, 6 * MAN];
    else actual = [3 * MAN, 3 * MAN];
  }
  const lostIt = Math.max(0, full[0] - actual[0]);
  const lostRt = Math.max(0, full[1] - actual[1]);
  const increase = floorTo(Math.floor((lostIt * 1021) / 10000) + Math.floor(lostRt / 10), 100);

  if (value <= line) {
    const room = line - value;
    const isTight = room <= 10 * MAN;
    return {
      status: isTight ? 'caution' : 'safe',
      label: isTight ? 'ギリギリ' : 'セーフ',
      value,
      line,
      message: `所得${fmtYen(value)}は${year}年のライン${fmtYen(line)}以内。あと${fmtYen(room + 1)}増えるとアウト。`,
      parentTaxIncrease: 0,
    };
  }
  if (specific && value <= 123 * MAN) {
    return {
      status: 'caution',
      label: '一部アウト',
      value,
      line,
      message: `所得${fmtYen(value)}で${fmtYen(line)}を超えたため、親の控除が「特定扶養63万円」→「特定親族特別控除${fmtYen(actual[0])}」に減ります。親の年末調整・確定申告の書類変更が必要で、親の税金が年 約${fmtYen(increase)}増えます。`,
      parentTaxIncrease: increase,
    };
  }
  return {
    status: 'out',
    label: 'アウト',
    value,
    line,
    message: `所得${fmtYen(value)}がライン${fmtYen(specific ? 123 * MAN : line)}を超えて親の扶養から外れます。親が扶養控除を使っていたら年末調整・確定申告のやり直しが必要で、親側に約${fmtYen(increase)}の追加納税が発生します。`,
    parentTaxIncrease: increase,
  };
}

function judgeInsuranceDependent(
  age: number,
  annualEst: number,
  paceEst: number,
  ownShaho: boolean
): DependentJudge {
  const line = age >= 19 && age <= 22 ? 150 * MAN : 130 * MAN;
  const value = Math.max(0, annualEst);
  if (ownShaho) {
    return {
      status: 'na',
      label: '自分で加入',
      value,
      line,
      message: '勤務先の健康保険に自分で入っているので、親の健康保険の扶養は関係ありません。',
      parentTaxIncrease: 0,
    };
  }
  const lineTxt = fmtYen(line);
  if (value >= line) {
    return {
      status: 'out',
      label: 'アウト',
      value,
      line,
      message: `年間収入の見込み${fmtYen(value)}が${lineTxt}以上。親の保険証から抜けて国保へ切り替えが必要。抜けるべき日以降に親の保険で病院にかかった分は、医療費（7割分）の返還を求められるリスクがあります。`,
      parentTaxIncrease: 0,
    };
  }
  if (paceEst >= line) {
    return {
      status: 'caution',
      label: 'グレー',
      value,
      line,
      message: `年合計${fmtYen(value)}はライン未満でも、月平均ペースだと年換算${fmtYen(paceEst)}。健康保険は「今後1年の見込み」で判断するため、始めた月から扶養外と見なされる可能性があります（加入先の健保組合に確認）。`,
      parentTaxIncrease: 0,
    };
  }
  if (value >= Math.floor((line * 85) / 100)) {
    return {
      status: 'caution',
      label: 'ギリギリ',
      value,
      line,
      message: `年間収入の見込み${fmtYen(value)}。あと${fmtYen(line - value)}で${lineTxt}の壁を超えて扶養から外れます。`,
      parentTaxIncrease: 0,
    };
  }
  return {
    status: 'safe',
    label: 'セーフ',
    value,
    line,
    message: `年間収入の見込み${fmtYen(value)}は${lineTxt}未満。親の保険証のままでOK。`,
    parentTaxIncrease: 0,
  };
}

function rankStatus(s: Status): number {
  return s === 'out' ? 3 : s === 'caution' ? 2 : s === 'safe' ? 1 : 0;
}

function maxStatus(a: Status, b: Status): Status {
  return rankStatus(a) >= rankStatus(b) ? a : b;
}

/** Rust エンジンと完全同一のフォールバック診断実行 */
function diagnoseFallback(input: DiagnoseInput): DiagnoseOutput {
  const pref = NHI_TABLE[input.prefecture] ?? NHI_TABLE[47];
  const age2026 = Math.min(80, Math.max(12, input.age));
  const age2025 = Math.max(0, age2026 - 1);

  const calcYear = (year: number, age: number, prevYearIncome?: number): YearResult => {
    const grossRaw = year === 2025 ? input.income2025 : input.income2026;
    const expRaw = year === 2025 ? input.expense2025 : input.expense2026;
    const gross = Math.max(0, grossRaw);
    const expense = Math.min(gross, Math.max(0, expRaw));
    const profitBeforeBlue = Math.max(0, gross - expense);
    const blueDeduction = input.blueReturn ? Math.min(profitBeforeBlue, 65 * MAN) : 0;
    const businessIncome = Math.max(0, profitBeforeBlue - blueDeduction);

    const salaryRaw = input.hasSalary ? Math.max(0, input.salaryAnnual) : 0;
    const salIt = salaryIncomeIt(year, salaryRaw);
    const salRt = salaryIncomeRt(year, salaryRaw);
    const totalIncomeIt = businessIncome + salIt;
    const totalIncomeRt = businessIncome + salRt;

    const taxDep = judgeTaxDependent(year, age, totalIncomeIt);
    const annualEst = profitBeforeBlue + salaryRaw;
    const paceEst =
      year === 2025 && input.startMonth2025 >= 1 && input.startMonth2025 <= 12
        ? Math.floor((profitBeforeBlue * 12) / Math.max(1, 13 - input.startMonth2025)) + salaryRaw
        : annualEst;
    const insDep = judgeInsuranceDependent(age, annualEst, paceEst, input.salaryHasShaho);

    const needsOwnNhi =
      !input.salaryHasShaho &&
      (insDep.status === 'out' || (input.livesAlone && annualEst >= 130 * MAN));
    const nhiCalc = needsOwnNhi
      ? calcNhi(input.prefecture, totalIncomeRt, age, input.livesAlone, input.hasSalary)
      : { premium: 0, reduction: 0 };

    const prevInc = prevYearIncome ?? totalIncomeIt;
    const judgeInc = Math.max(totalIncomeIt, prevInc);
    const canDeferOrExempt =
      age >= 19 && age <= 22 ? judgeInc <= 128 * MAN : judgeInc <= 67 * MAN;
    const pensionAmt =
      (gross === 0 && salaryRaw === 0) || canDeferOrExempt
        ? 0
        : age >= 20 && age < 60 && !input.salaryHasShaho
          ? (year >= 2026 ? 17920 : 17510) * 12
          : 0;
    const socialDeduction = nhiCalc.premium + pensionAmt;

    const basic = basicDeduction(year, totalIncomeIt);
    const taxableIncome = floorTo(totalIncomeIt - basic - socialDeduction, 1000);
    const itFull = taxOnTaxableIncome(taxableIncome);
    const itSalOnly = input.hasSalary
      ? taxOnTaxableIncome(floorTo(salIt - basicDeduction(year, salIt), 1000))
      : 0;
    const incomeTaxAmt = Math.min(profitBeforeBlue, Math.max(0, itFull - itSalOnly));

    const rtFull = calcResidentTax(totalIncomeRt, socialDeduction, age + 1);
    const rtSalOnly = input.hasSalary ? calcResidentTax(salRt, 0, age + 1) : 0;
    const residentTaxAmt = Math.min(profitBeforeBlue, Math.max(0, rtFull - rtSalOnly));

    const filingRequired =
      input.hasSalary && salaryRaw > 0
        ? businessIncome > 20 * MAN || (input.blueReturn && profitBeforeBlue > 20 * MAN)
        : totalIncomeIt > basic ||
          (input.blueReturn && profitBeforeBlue > basic) ||
          (year === 2025 && profitBeforeBlue > 48 * MAN);

    const residentFilingRequired = !filingRequired && totalIncomeRt > 45 * MAN;
    const fy = year + 1;

    return {
      year,
      age,
      gross,
      expense,
      blueDeduction,
      businessIncome,
      salaryIncome: salIt,
      totalIncome: totalIncomeIt,
      basicDeduction: basic,
      socialDeduction,
      taxableIncome,
      incomeTax: incomeTaxAmt,
      residentTax: residentTaxAmt,
      residentTaxLabel: `令和${fy - 2018}年度（${fy}年6月〜）`,
      nhi: nhiCalc.premium,
      nhiLabel: `令和${fy - 2018}年度（${fy}年4月〜${fy + 1}年3月）`,
      nhiReduction: nhiCalc.reduction,
      pension: pensionAmt,
      total: incomeTaxAmt + residentTaxAmt + nhiCalc.premium + pensionAmt,
      taxDependent: taxDep,
      insuranceDependent: insDep,
      filingRequired,
      residentFilingRequired,
    };
  };

  const y2025 = calcYear(2025, age2025);
  const y2026 = calcYear(2026, age2026, y2025.totalIncome);

  // 2025年期限後申告チェック
  let lateFiling: LateFiling | null = null;
  if (!input.filed2025) {
    const profit2025 = Math.max(0, y2025.gross - y2025.expense);
    const threshold = input.hasSalary && input.salaryAnnual > 0 ? 20 * MAN : 48 * MAN;
    if (profit2025 > threshold || y2025.filingRequired) {
      const tax = y2025.incomeTax;
      const days = Math.max(0, input.daysSince2025Deadline);
      const rawPenalty = floorTo(Math.floor((tax * 5) / 100), 100);
      const penalty = rawPenalty < 5000 ? 0 : rawPenalty;
      const taxBase = floorTo(tax, 10000);
      let delinquency = 0;
      if (taxBase >= 10000 && days > 0) {
        const d1 = Math.min(61, days);
        const d2 = Math.max(0, days - 61);
        const interest =
          Math.floor((taxBase * 28 * d1) / 365000) + Math.floor((taxBase * 91 * d2) / 365000);
        const rounded = floorTo(interest, 100);
        delinquency = rounded < 1000 ? 0 : rounded;
      }
      const message =
        tax > 0
          ? `2025年（昨年）分の所得が${fmtYen(y2025.businessIncome)}あり、確定申告期限（2026年3月16日）を過ぎています！税務署から連絡が来る前に自主的に「期限後申告」すれば、無申告加算税は5%（約${fmtYen(penalty)}）に軽減されます（指摘後だと15〜20%）。`
          : `2025年（昨年）分の配信所得が${fmtYen(profit2025)}（48万円超）あります。基礎控除の引き上げで所得税が0円でも、住民税や親の扶養判定のために申告が必要です。早めに期限後申告（または市区町村への住民税申告）を出しましょう。`;
      lateFiling = { tax, penalty, delinquency, message };
    }
  }

  const unpaid2025It = input.filed2025
    ? 0
    : y2025.incomeTax + (lateFiling ? lateFiling.penalty + lateFiling.delinquency : 0);
  const lockTotal = y2026.total + unpaid2025It + y2025.residentTax;
  const net2026 = Math.max(0, y2026.gross - y2026.expense);
  const freeMoney = Math.max(0, net2026 - y2026.total);
  const setAsidePercent =
    y2026.gross > 0
      ? Math.min(60, Math.max(5, Math.ceil((y2026.total * 100) / y2026.gross)))
      : y2025.gross > 0
        ? Math.min(60, Math.max(5, Math.ceil((y2025.total * 100) / y2025.gross)))
        : 0;

  let overall: Status = 'safe';
  for (const y of [y2025, y2026]) {
    overall = maxStatus(overall, maxStatus(y.taxDependent.status, y.insuranceDependent.status));
  }
  if (lateFiling || (!input.filed2025 && y2025.filingRequired)) {
    overall = 'out';
  }

  let headline = '';
  if (lateFiling) {
    headline = '🚨 昨年（2025年）分の無申告アラートあり！今すぐ期限後申告＆親への連絡が必要です';
  } else if (overall === 'out') {
    headline =
      y2026.insuranceDependent.status === 'out'
        ? '🔴 親の「税金の扶養」も「健康保険証」もアウト！保険証の切り替えと納税資金の確保を'
        : '🔴 親の「税金の扶養」から外れます！年末調整の伝え直しと納税資金の確保を';
  } else if (overall === 'caution') {
    headline = '🟡 扶養の壁ギリギリ（または特定親族特別控除ゾーン）！年末までの振込ペースに注意';
  } else {
    headline =
      y2025.gross === 0 && y2026.gross === 0
        ? '🎙️ 上のマイクで喋るかスライダーを動かすと、税金・国保・親の扶養を一撃診断します'
        : '🟢 親の扶養も保険証もセーフ！領収書・スクショだけ残しておけば安心です';
  }

  const actions: Action[] = [];
  if (lateFiling) {
    actions.push({
      priority: 'out',
      icon: '🚨',
      title: '昨年（2025年）分の「期限後申告」を今すぐ出す',
      detail: `${lateFiling.message} マイナンバーカード＋スマホ（e-Tax）で自宅から提出できます。`,
      when: '今すぐ（1日でも早く）',
    });
  }
  if (y2025.insuranceDependent.status === 'out' || y2026.insuranceDependent.status === 'out') {
    actions.push({
      priority: 'out',
      icon: '🪪',
      title: '親に健康保険証の件を話し、市区町村役場で「国民健康保険」に切り替える',
      detail:
        '親の勤務先に「被扶養者異動届」を出して資格喪失証明書をもらい、14日以内に役所の国保窓口（またはマイナポータル）で加入手続きをします。放置すると後から病院代7割分の返還請求が親に届きます。',
      when: '収入見込みが壁を超えた時点から14日以内',
    });
  } else if (
    y2025.insuranceDependent.status === 'caution' ||
    y2026.insuranceDependent.status === 'caution'
  ) {
    actions.push({
      priority: 'caution',
      icon: '🪪',
      title: '親の健康保険組合の「扶養ルール（自営業の経費の扱い）」を確認する',
      detail:
        '健康保険組合によっては「経費を引く前の売上（振込額）」で130万/150万を判定するところがあります。親の保険証に記載の健保組合名をググるか親に確認しましょう。',
      when: '今月中',
    });
  }
  if (y2025.taxDependent.status === 'out' || y2026.taxDependent.status === 'out') {
    const incMax = Math.max(
      y2025.taxDependent.parentTaxIncrease,
      y2026.taxDependent.parentTaxIncrease
    );
    actions.push({
      priority: 'out',
      icon: '👪',
      title: '親に「今年の年末調整で私を扶養から外して」とLINEする',
      detail: `この診断カードをスクショして親に送ればOK！親の税金が年 約${fmtYen(incMax)}増えるので、その分を配信の稼ぎから親に渡すと揉めません。`,
      when: '親の会社の年末調整（11〜12月）の前まで',
    });
  } else if (
    y2025.taxDependent.status === 'caution' ||
    y2026.taxDependent.status === 'caution'
  ) {
    actions.push({
      priority: 'caution',
      icon: '👪',
      title: '親に「今年の所得の見込み額」を共有しておく（特定親族特別控除）',
      detail:
        '19〜22歳は所得62万円（2025年は58万円）を超えても123万円以下なら「特定親族特別控除」で親の控除が段階的に残りますが、親が年末調整の紙にあなたの所得見込みを正確に書く必要があります。',
      when: '11月〜12月（親の年末調整時）',
    });
  }
  const profit2026 = Math.max(0, y2026.gross - y2026.expense);
  if (!input.blueReturn && profit2026 > 65 * MAN) {
    actions.push({
      priority: 'caution',
      icon: '📘',
      title: '「開業届」＋「青色申告承認申請書」を出して65万円控除を使う',
      detail:
        'スマホ会計アプリ（freee・マネーフォワード・弥生など）から無料で書類を作ってマイナポータル送信できます。最大65万円を所得から引けるので、税金・国保が一気に安くなり親の扶養にも残りやすくなります。',
      when: '青色にしたい年の3月15日まで（新規開業は開業から2か月以内）',
    });
  }
  if (y2026.pension > 0) {
    const hint =
      y2026.age >= 19 && y2026.age <= 22 && y2025.totalIncome <= 128 * MAN
        ? '学生なら「学生納付特例」で支払いを後回しにできる可能性あり（前年所得128万円以下が目安）。役所の年金窓口かマイナポータルで申請。'
        : y2025.totalIncome <= 67 * MAN
          ? '前年の所得が67万円以下なら「全額免除」になる可能性あり（本人・世帯主の所得で判定）。未納のまま放置せず、必ず免除申請を。'
          : '所得的に免除はむずかしいライン。1年前納（口座振替）にすると少し割引になる。払った年金は全額「社会保険料控除」で税金が安くなる。';
    actions.push({
      priority: 'safe',
      icon: '🪙',
      title: '20歳以上の「国民年金」の手続き・学生納付特例をチェック',
      detail: hint,
      when: '20歳の誕生月以降／毎年4月',
    });
  }
  if (y2026.residentFilingRequired) {
    actions.push({
      priority: 'caution',
      icon: '🏛️',
      title: '所得税は0円でも「住民税の申告」を忘れずに',
      detail:
        '2026年は基礎控除104万円のおかげで所得税が0円でも、所得が45万円を超えているため住民税の申告が必要です（確定申告を出せば住民税申告も兼ねられます）。',
      when: '2027年2月16日〜3月15日',
    });
  }
  actions.push({
    priority: 'safe',
    icon: '🧾',
    title: '配信で使ったお金の「スクショ・レシート」をスマホのアルバムに保存',
    detail:
      'リングライト・マイク・スマホスタンド・配信用の衣装やコスメ・BGMやスタンプ代・コラボ配信の交通費・自宅の家賃や通信費の一部（按分）は経費になります。経費が増えるほど所得が減って税金も扶養判定も有利に！',
    when: '今日から毎日',
  });

  const notes: string[] = [
    `国保料は「${pref.name}（${pref.city}）」の40歳未満・介護分なし代表料率（令和8年度・子ども子育て支援金分含む）による概算です。実際の金額はお住まいの市区町村により異なります。`,
    '2025年分は令和7年度税制（基礎控除58万〜最大95万）、2026年分は令和8年度税制改正（基礎控除 本則62万・特例最大104万、扶養の所得要件62万、給与所得控除最低74万）を適用しています。',
    '親の増税額は「親の所得税率10%＋住民税率10%」の一般的な世帯を想定した目安です（親の年収が高い場合はさらに増えることがあります）。',
  ];
  if (!input.livesAlone) {
    notes.push(
      '親と同居（親が世帯主）として国保の低所得軽減は「なし」で保守的に計算しています（一人暮らしで自分が世帯主の場合は2〜7割軽減が効くことがあります）。'
    );
  }

  return {
    engineVersion: '2025-2026.r1',
    prefectureName: pref.name,
    prefectureCity: pref.city,
    years: [y2025, y2026],
    lateFiling,
    lockTotal,
    freeMoney,
    setAsidePercent,
    overall,
    headline,
    actions,
    notes,
  };
}

let wasmDiagnoseFn: ((json: string) => string) | null = null;
let wasmMode: 'wasm' | 'fallback' = 'fallback';

/** Wasm モジュールの初期化を試みる（無ければ即座に内蔵エンジンへフォールバック） */
export async function initTaxEngine(): Promise<'wasm' | 'fallback'> {
  try {
    const modUrl = new URL('./pkg/rust_tax_engine.js', import.meta.url).href;
    const wasmMod = await import(/* @vite-ignore */ modUrl);
    if (typeof wasmMod.default === 'function') {
      await wasmMod.default();
    }
    if (typeof wasmMod.diagnose_json === 'function') {
      wasmDiagnoseFn = wasmMod.diagnose_json;
      wasmMode = 'wasm';
      return 'wasm';
    }
  } catch {
    // wasm-pack ビルド前の環境では内蔵エンジンを使用
  }
  wasmMode = 'fallback';
  return 'fallback';
}

export function getEngineMode(): 'wasm' | 'fallback' {
  return wasmMode;
}

export function runDiagnose(input: DiagnoseInput): DiagnoseOutput {
  if (wasmDiagnoseFn) {
    const outJson = wasmDiagnoseFn(JSON.stringify(input));
    return JSON.parse(outJson) as DiagnoseOutput;
  }
  return diagnoseFallback(input);
}

export function getPrefectureList(): PrefectureItem[] {
  return NHI_TABLE.map((p, id) => ({ id, name: p.name, city: p.city }));
}
