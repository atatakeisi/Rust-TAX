/**
 * 会計ソフト並みの「確定申告・税金管理台帳（仕訳帳・収支検証・科目別集計）」モジュール
 *
 * - 日々の売上・毎月の振込額・機材や衣装などの経費を日付つきで順番に入力
 * - ブラウザ内 localStorage に自動保存（外部サーバー送信ゼロ）
 * - 2025年分 / 2026年分の売上合計・経費合計・月別集計・勘定科目別集計をリアルタイム検証
 * - ワンタップで上の診断エンジン（スライダー＆税金・国保・扶養判定）へ反映
 * - CSV エクスポート（確定申告ソフトやExcelにそのまま貼れる形式）対応
 */

                                                   

                             
                      
               
                 
                      
                      
                       
                     
                        

                              
             
                   
               
                        
                            
              
                 
              
               
 

                                    
                    
                      
                       
                    
                                     
                                                                     
                   
                                                          
                  
                                               
 

const STORAGE_KEY = 'tiktok_liver_tax_ledger_v1';

export const INCOME_CATEGORIES                    = [
  '振込売上（TikTok・投げ銭）',
  'イベント・案件報酬',
];

export const EXPENSE_CATEGORIES                    = [
  '消耗品費（機材・ライト・マイク）',
  '衣装・美容費（配信用コスメ・服）',
  '通信費（スマホ・回線）',
  '地代家賃（配信部屋の家賃按分）',
  '旅費交通費（コラボ・イベント移動）',
  '接待交際費・雑費（企画・BGM等）',
];

/** 初回表示用のサンプル台帳データ（日付リンク付き） */
const DEFAULT_ENTRIES                = [
  {
    id: 'sample-1',
    date: '2025-10-31',
    type: 'income',
    category: '振込売上（TikTok・投げ銭）',
    amount: 150_000,
    memo: '10月分 TikTokダイヤモンド換金振込',
  },
  {
    id: 'sample-2',
    date: '2025-11-30',
    type: 'income',
    category: '振込売上（TikTok・投げ銭）',
    amount: 170_000,
    memo: '11月分 TikTok振込',
  },
  {
    id: 'sample-3',
    date: '2025-12-28',
    type: 'income',
    category: '振込売上（TikTok・投げ銭）',
    amount: 180_000,
    memo: '12月分 TikTok振込（年末イベント）',
  },
  {
    id: 'sample-4',
    date: '2026-01-25',
    type: 'expense',
    category: '消耗品費（機材・ライト・マイク）',
    amount: 85_000,
    memo: '配信用コンデンサーマイク＆リングライト一式',
  },
  {
    id: 'sample-5',
    date: '2026-02-28',
    type: 'income',
    category: '振込売上（TikTok・投げ銭）',
    amount: 450_000,
    memo: '1〜2月分 TikTok定期振込',
  },
  {
    id: 'sample-6',
    date: '2026-04-15',
    type: 'expense',
    category: '衣装・美容費（配信用コスメ・服）',
    amount: 65_000,
    memo: '生誕イベント配信用の衣装・ヘアメイク代',
  },
  {
    id: 'sample-7',
    date: '2026-06-30',
    type: 'income',
    category: '振込売上（TikTok・投げ銭）',
    amount: 950_000,
    memo: '3〜6月分 TikTok振込合計',
  },
  {
    id: 'sample-8',
    date: '2026-09-10',
    type: 'expense',
    category: '通信費（スマホ・回線）',
    amount: 72_000,
    memo: '光回線＋スマホ通信費（配信按分60%・年額）',
  },
  {
    id: 'sample-9',
    date: '2026-10-31',
    type: 'income',
    category: '振込売上（TikTok・投げ銭）',
    amount: 1_100_000,
    memo: '7〜10月分 TikTok振込合計',
  },
  {
    id: 'sample-10',
    date: '2026-11-05',
    type: 'expense',
    category: '地代家賃（配信部屋の家賃按分）',
    amount: 78_000,
    memo: '防音マット・配信スペース家賃按分',
  },
];

export function loadLedger()                {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [...DEFAULT_ENTRIES];
    const parsed = JSON.parse(raw)                 ;
    if (Array.isArray(parsed)) return parsed;
  } catch {
    // localStorage 無効時はメモリ上のデフォルトを返す
  }
  return [...DEFAULT_ENTRIES];
}

export function saveLedger(entries               )       {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(entries));
  } catch {
    // ignore
  }
}

export function summarizeLedgerByYear(
  entries               ,
  year             ,
  blueReturn         
)                    {
  const prefix = `${year}-`;
  const filtered = entries
    .filter((e) => e.date.startsWith(prefix))
    .sort((a, b) => a.date.localeCompare(b.date));

  let totalIncome = 0;
  let totalExpense = 0;

  const monthly = Array.from({ length: 12 }, (_, i) => ({
    month: i + 1,
    income: 0,
    expense: 0,
  }));

  const catMap = new Map                ();

  for (const e of filtered) {
    const m = parseInt(e.date.slice(5, 7), 10);
    const mIdx = m >= 1 && m <= 12 ? m - 1 : 0;
    if (e.type === 'income') {
      totalIncome += e.amount;
      monthly[mIdx].income += e.amount;
    } else {
      totalExpense += e.amount;
      monthly[mIdx].expense += e.amount;
      catMap.set(e.category, (catMap.get(e.category) ?? 0) + e.amount);
    }
  }

  const byCategory = Array.from(catMap.entries())
    .map(([category, amount]) => ({ category, amount }))
    .sort((a, b) => b.amount - a.amount);

  const netProfit = Math.max(0, totalIncome - totalExpense);
  const expenseRatio = totalIncome > 0 ? Math.round((totalExpense * 100) / totalIncome) : 0;

  // 確定申告の自動検証チェック
  const checks                                       = [
    {
      ok: filtered.length > 0,
      text: `${year}年の記帳件数: ${filtered.length}件（日付順ソート済）`,
    },
    {
      ok: totalExpense <= totalIncome,
      text:
        totalExpense <= totalIncome
          ? `収支バランス正常（経費率 ${expenseRatio}% / 利益 ${(netProfit / 10000).toFixed(1)}万円）`
          : '⚠️ 経費が売上を上回っています（赤字繰越には青色申告が必要です）',
    },
    {
      ok: blueReturn || netProfit <= 650_000,
      text: blueReturn
        ? '📘 青色申告（65万円特別控除）適用対象：複式簿記・損益計算書用データ整理済'
        : '📄 白色申告モード：事業者登録（青色申告）に変えると最大65万円所得を減らせます',
    },
  ];

  return {
    year,
    totalIncome,
    totalExpense,
    netProfit,
    monthly,
    byCategory,
    checks,
  };
}

/** 確定申告ソフト・Excel貼り付け用 CSV 文字列を生成 */
export function exportLedgerCsv(entries               )         {
  const sorted = [...entries].sort((a, b) => a.date.localeCompare(b.date));
  const header = '日付,年度,区分,勘定科目,金額(円),摘要・メモ';
  const rows = sorted.map((e) => {
    const yr = e.date.slice(0, 4);
    const kind = e.type === 'income' ? '売上(収入)' : '経費(支出)';
    const safeMemo = `"${(e.memo || '').replace(/"/g, '""')}"`;
    return `${e.date},${yr}年,${kind},${e.category},${e.amount},${safeMemo}`;
  });
  return '\uFEFF' + [header, ...rows].join('\n');
}


//# sourceURL=ledger.ts