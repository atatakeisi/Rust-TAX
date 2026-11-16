/**
 * TikTokライバー向け「税金・国保・親の扶養」一撃診断＆確定申告管理台帳 フロントエンド制御
 *
 * - 初期地域設定は身バレ防止のため「全国平均（インデックス47）」をデフォルト採用（台帳エリアで47都道府県別に切替可能）
 * - 録音＋最小プロフィール直後に「所得税・住民税・国民健康保険料」の3大数値を特大表示
 * - 「事業者登録した場合（青色・65万控除）」と「しない場合（白色・0円）」を色分けボタンで一発比較・切替
 * - すべてのスライダーに「− / ＋」ボタンを併設し、動かせることに一目で気付けるUX
 * - 収入を増やせば「手元に残る安全なお金」が必ず増える（低所得・学生の年金猶予/免除考慮）
 * - 普段の微調整スライダーや詳細内訳は `<details>` で格納し、展開ボタンでいつでも開閉可能
 * - 最下部に日付連動の「確定申告・税金管理台帳（仕訳入力・科目別集計・月次推移・CSV出力）」を搭載
 */

import {
  EXPENSE_CATEGORIES,
                       
  INCOME_CATEGORIES,
                   
                       
  exportLedgerCsv,
  loadLedger,
  saveLedger,
  summarizeLedgerByYear,
} from './ledger.js';
import { parseVoiceText } from './voice_parser.js';
import {
                     
                      
              
                  
  getEngineMode,
  getPrefectureList,
  initTaxEngine,
  runDiagnose,
} from './wasm_bridge.js';

const MAN = 10_000;
const NATIONAL_AVERAGE_PREF = 47;

/** 2025年分確定申告期限（2026-03-16）からの経過日数を算出 */
function calcDaysSince2025Deadline()         {
  const deadline = new Date('2026-03-16T00:00:00+09:00').getTime();
  const now = Date.now();
  if (now <= deadline) return 0;
  return Math.floor((now - deadline) / (1000 * 60 * 60 * 24));
}

/** 現在の入力ステート（初期地域は身バレ防止の全国平均＝47） */
const state                = {
  age: 20,
  prefecture: NATIONAL_AVERAGE_PREF,
  income2025: 50 * MAN,
  expense2025: 0,
  income2026: 250 * MAN,
  expense2026: 30 * MAN,
  hasSalary: false,
  salaryAnnual: 0,
  salaryHasShaho: false,
  blueReturn: false,
  filed2025: false,
  livesAlone: false,
  startMonth2025: 10,
  daysSince2025Deadline: calcDaysSince2025Deadline(),
};

/** 確定申告台帳ステート */
let ledgerEntries                = [];
let activeLedgerYear              = 2026;

/** 金額フォーマット（万円表示＋円表示） */
function fmtMan(yen        )         {
  const v = Math.max(0, Math.round(yen));
  if (v === 0) return '0円';
  if (v >= MAN) {
    const tenths = Math.round(v / 1000);
    const whole = Math.floor(tenths / 10);
    const frac = tenths % 10;
    return frac === 0
      ? `${whole.toLocaleString('ja-JP')}万円`
      : `${whole.toLocaleString('ja-JP')}.${frac}万円`;
  }
  return `${v.toLocaleString('ja-JP')}円`;
}

function fmtExactYen(yen        )         {
  return `${Math.max(0, Math.round(yen)).toLocaleString('ja-JP')}円`;
}

function statusBadgeClass(s        )         {
  switch (s) {
    case 'out':
      return 'badge-out';
    case 'caution':
      return 'badge-caution';
    case 'safe':
      return 'badge-safe';
    default:
      return 'badge-na';
  }
}

function statusIcon(s        )         {
  switch (s) {
    case 'out':
      return '🔴';
    case 'caution':
      return '🟡';
    case 'safe':
      return '🟢';
    default:
      return '⚪';
  }
}

function $                       (id        )    {
  return document.getElementById(id)     ;
}

/** UI のフォーム・スライダー・ボタン選択状態を `state` に合わせて同期 */
function syncControlsFromState()       {
  // 都道府県（台帳内の精密セレクト）
  const prefSelect = $                   ('pref-select');
  if (prefSelect) prefSelect.value = String(state.prefecture);

  // 年齢区分ボタン
  const ageGroup = state.age <= 18 ? 'u18' : state.age <= 22 ? 'student' : 'adult';
  document.querySelectorAll                   ('[data-age-group]').forEach((btn) => {
    btn.classList.toggle('active', btn.dataset.ageGroup === ageGroup);
  });

  // 本業・バイト給与ボタン
  document.querySelectorAll                   ('[data-has-salary]').forEach((btn) => {
    const val = btn.dataset.hasSalary === 'true';
    btn.classList.toggle('active', val === state.hasSalary);
  });
  const salaryBox = $('salary-detail-box');
  if (salaryBox) {
    salaryBox.classList.toggle('hidden', !state.hasSalary);
  }
  const salSlider = $                  ('slider-salary');
  const salVal = $('val-salary');
  if (salSlider) salSlider.value = String(Math.round(state.salaryAnnual / MAN));
  if (salVal) salVal.textContent = `${Math.round(state.salaryAnnual / MAN)}万円`;

  // 事業者登録（青色 / 白色）一発切替ボタン
  document.querySelectorAll                   ('[data-blue-return]').forEach((btn) => {
    const val = btn.dataset.blueReturn === 'true';
    btn.classList.toggle('active', val === state.blueReturn);
  });

  // 2025年スライダー
  const inc25Slider = $                  ('slider-inc-2025');
  const exp25Slider = $                  ('slider-exp-2025');
  if (inc25Slider) inc25Slider.value = String(Math.round(state.income2025 / MAN));
  if (exp25Slider) exp25Slider.value = String(Math.round(state.expense2025 / MAN));
  $('val-inc-2025').textContent = `${Math.round(state.income2025 / MAN)}万円`;
  $('val-exp-2025').textContent = `${Math.round(state.expense2025 / MAN)}万円`;

  const startMonthSelect = $                   ('select-start-2025');
  if (startMonthSelect) startMonthSelect.value = String(state.startMonth2025);

  const filed25Check = $                  ('check-filed-2025');
  if (filed25Check) filed25Check.checked = state.filed2025;

  // 2026年スライダー
  const inc26Slider = $                  ('slider-inc-2026');
  const exp26Slider = $                  ('slider-exp-2026');
  if (inc26Slider) inc26Slider.value = String(Math.round(state.income2026 / MAN));
  if (exp26Slider) exp26Slider.value = String(Math.round(state.expense2026 / MAN));
  $('val-inc-2026').textContent = `${Math.round(state.income2026 / MAN)}万円`;
  $('val-exp-2026').textContent = `${Math.round(state.expense2026 / MAN)}万円`;
}

/** 現在の `state` から、STEP 1 のサマリーチップ（#detected-tags）と音声テキスト欄を常に最新状態に同期する */
function renderStateSummaryTags(out                , syncTranscriptText = false)       {
  const ageGroupTxt =
    state.age <= 18
      ? `🎂 ${state.age}歳（18歳以下）`
      : state.age <= 22
        ? `🎂 ${state.age}歳（19〜22歳）`
        : `🎂 ${state.age}歳（23歳以上）`;
  const prefTxt =
    state.prefecture === NATIONAL_AVERAGE_PREF
      ? '🛡️ 全国平均（匿名モード）'
      : `📍 ${out.prefectureName}`;

  const chips           = [prefTxt, ageGroupTxt];
  if (state.startMonth2025 >= 1 && state.startMonth2025 <= 12) {
    chips.push(`📅 昨年${state.startMonth2025}月開始`);
  }
  chips.push(`💴 2025年振込 ${fmtMan(state.income2025)}`);
  if (state.expense2025 > 0) {
    chips.push(`🧾 2025年経費 ${fmtMan(state.expense2025)}`);
  }
  chips.push(`💴 2026年振込 ${fmtMan(state.income2026)}`);
  if (state.expense2026 > 0) {
    chips.push(`🧾 2026年経費 ${fmtMan(state.expense2026)}`);
  }
  if (state.hasSalary) {
    chips.push(`💼 給与 年${fmtMan(state.salaryAnnual)}`);
  }
  if (state.blueReturn) {
    chips.push('📘 青色申告（65万控除）');
  }

  const tagsBox = $('detected-tags');
  if (tagsBox) {
    tagsBox.innerHTML = chips.map((t) => `<span class="detected-chip">${t}</span>`).join('');
    tagsBox.classList.remove('hidden');
  }

  // 手入力スライダーやボタンで変更されたとき、認識テキスト欄にフォーカスしていなければテキスト欄の文言も同期
  if (syncTranscriptText) {
    const transcriptBox = $                  ('voice-transcript-input');
    if (transcriptBox && document.activeElement !== transcriptBox) {
      const parts           = [`${state.age}歳`];
      if (state.prefecture !== NATIONAL_AVERAGE_PREF) {
        parts.push(out.prefectureName);
      }
      if (state.hasSalary) {
        parts.push(`バイト年${Math.round(state.salaryAnnual / MAN)}万`);
      }
      const startTxt =
        state.startMonth2025 >= 1 && state.startMonth2025 <= 12
          ? `去年${state.startMonth2025}月から始めて`
          : '去年は';
      parts.push(
        `${startTxt}${Math.round(state.income2025 / MAN)}万${
          state.expense2025 > 0 ? `（去年の経費${Math.round(state.expense2025 / MAN)}万）` : ''
        }`
      );
      parts.push(`今年は${Math.round(state.income2026 / MAN)}万振り込まれて`);
      parts.push(`経費は${Math.round(state.expense2026 / MAN)}万くらい`);
      if (state.blueReturn) {
        parts.push('青色申告');
      }
      transcriptBox.value = parts.join('、');
    }
  }
}

/** 診断を実行し、結果カード全体（＋青色と白色の差額比較＋上部タグ）を再描画 */
function recalculateAndRender(syncTranscriptText = true)       {
  const out                 = runDiagnose(state);
  renderStateSummaryTags(out, syncTranscriptText);
  // 青色 vs 白色の節税差額をリアルタイム比較するため、反対側のモードも裏で計算
  const outWhite                 = state.blueReturn
    ? runDiagnose({ ...state, blueReturn: false })
    : out;
  const outBlue                 = state.blueReturn
    ? out
    : runDiagnose({ ...state, blueReturn: true });

  const [y25, y26] = out.years;

  // 1. 結果カードのカラーテーマ（blue / white）と総合ステータスを反映
  const card = $('result-card');
  card.dataset.overall = out.overall;
  card.dataset.bizMode = state.blueReturn ? 'blue' : 'white';
  $('result-headline').textContent = out.headline;

  const ageLabel =
    state.age <= 18
      ? `${state.age}歳（高校生等）`
      : state.age <= 22
        ? `${state.age}歳（学生世代）`
        : `${state.age}歳`;
  const prefDisplay =
    state.prefecture === NATIONAL_AVERAGE_PREF
      ? '🛡️ 全国平均（匿名）'
      : `📍 ${out.prefectureName}（${out.prefectureCity}）`;
  $('result-meta').textContent = `${prefDisplay} ／ 🎂 ${ageLabel}`;

  // 2. 事業者登録（青色 vs 白色）一発比較バナーの更新
  const saving2026 = Math.max(0, outWhite.years[1].total - outBlue.years[1].total);
  const savingLock = Math.max(0, outWhite.lockTotal - outBlue.lockTotal);
  const savingBanner = $('biz-saving-banner');

  if (state.blueReturn) {
    savingBanner.innerHTML =
      savingLock > 0
        ? `🎉 <strong>青色申告（65万控除）</strong>により、白色と比べて<strong>約${fmtMan(savingLock)}（${fmtExactYen(savingLock)}）</strong>負担が減り、手元に残るお金が増えています！`
        : '📘 青色申告（最大65万円控除）で計算中（元々の所得が非課税圏内のため差額は0円です）。';
  } else {
    savingBanner.innerHTML =
      saving2026 > 0
        ? `💡 右の<strong>「📘 事業者登録した場合」</strong>をタップすると、税金・国保が<strong>約${fmtMan(savingLock)}（${fmtExactYen(savingLock)}）</strong>安くなります！`
        : '💡 右の「📘 事業者登録した場合」で青色申告（65万控除）に一発変換できます。';
  }

  // 3. 超特大3大コスト（所得税・住民税・国民健康保険料）の表示
  $('hero-income-pill').textContent =
    y26.blueDeduction > 0
      ? `2026年: 振込${fmtMan(y26.gross)} − 経費${fmtMan(y26.expense)} − 青色${fmtMan(y26.blueDeduction)} ＝ 所得${fmtMan(y26.businessIncome)}`
      : `2026年: 振込${fmtMan(y26.gross)} − 経費${fmtMan(y26.expense)} ＝ 所得${fmtMan(y26.businessIncome)}`;
  $('hero-it-2026').textContent = fmtMan(y26.incomeTax);
  $('hero-it-sub').textContent = `基礎控除${fmtMan(y26.basicDeduction)} / 課税所得${fmtMan(y26.taxableIncome)}`;
  $('hero-rt-2026').textContent = fmtMan(y26.residentTax);
  $('hero-rt-sub').textContent = y26.residentTax > 0 ? '翌年6月〜通知' : '非課税（0円）';
  $('hero-nhi-2026').textContent = y26.nhi > 0 ? fmtMan(y26.nhi) : '0円';
  $('hero-nhi-sub').textContent =
    y26.nhi > 0
      ? `${state.prefecture === NATIONAL_AVERAGE_PREF ? '全国平均目安' : out.prefectureName}`
      : '親の保険証（扶養内）で0円';

  // 4. 手元に残る安全なお金 ＆ 役所から分割で届く納付書の月別・回数別備えプラン
  const monthlyReserve = Math.ceil(out.lockTotal / 12 / 100) * 100;
  $('reserve-monthly-amount').textContent = `月 約${fmtMan(monthlyReserve)}`;
  $('lock-total-amount').textContent = fmtMan(out.lockTotal);
  $('lock-total-exact').textContent = `（${fmtExactYen(out.lockTotal)}）`;
  $('free-money-amount').textContent = fmtMan(out.freeMoney);
  $('free-money-exact').textContent = `（${fmtExactYen(out.freeMoney)}）`;
  $('aside-percent').textContent = `${out.setAsidePercent}%`;

  // 時期別の納付書スケジュール分解（3月所得税・6月〜年4回住民税・6月〜年10回国保）
  const rtPerInstallment = y26.residentTax > 0 ? Math.ceil(y26.residentTax / 4 / 100) * 100 : 0;
  const nhiPerInstallment = y26.nhi > 0 ? Math.ceil(y26.nhi / 10 / 100) * 100 : 0;

  $('sched-it').textContent = y26.incomeTax > 0 ? `約${fmtMan(y26.incomeTax)}（年1回）` : '0円';
  $('sched-rt').textContent =
    rtPerInstallment > 0 ? `1回 約${fmtMan(rtPerInstallment)}（6・8・10・1月）` : '0円';
  $('sched-nhi').textContent =
    nhiPerInstallment > 0 ? `1回 約${fmtMan(nhiPerInstallment)}（6〜3月の10回）` : '0円（扶養内）';

  // 5. 期限後申告バナー（2025年無申告アラート）
  const lateBox = $('late-filing-banner');
  if (out.lateFiling) {
    lateBox.classList.remove('hidden');
    $('late-filing-msg').textContent = out.lateFiling.message;
    const extra = out.lateFiling.penalty + out.lateFiling.delinquency;
    $('late-filing-numbers').textContent =
      out.lateFiling.tax > 0
        ? `2025年所得税 ${fmtExactYen(out.lateFiling.tax)} ＋ 無申告加算税・延滞税 約${fmtExactYen(extra)}`
        : '所得税0円（住民税・扶養判定のため期限後申告が必要）';
  } else {
    lateBox.classList.add('hidden');
  }

  // 6. 親の扶養アラート（クイックバー＆詳細ドロワー）
  renderDependentRow('2025', y25);
  renderDependentRow('2026', y26);

  // 7. 折りたたみドロワー内の年度別の所得ピル＆内訳
  $('net-2025').textContent = fmtMan(y25.businessIncome);
  $('net-2026').textContent = fmtMan(y26.businessIncome);
  $('blue-tag-2025').textContent =
    y25.blueDeduction > 0 ? `（青色 -${fmtMan(y25.blueDeduction)}適用後）` : '（白色 控除0円）';
  $('blue-tag-2026').textContent =
    y26.blueDeduction > 0 ? `（青色 -${fmtMan(y26.blueDeduction)}適用後）` : '（白色 控除0円）';

  renderTaxBreakdown('2026', y26);
  renderTaxBreakdown('2025', y25);

  // 8. アクションリスト＆注意書き
  const actionList = $('action-list');
  actionList.innerHTML = out.actions
    .map(
      (a) => `
      <li class="action-item priority-${a.priority}">
        <div class="action-header">
          <span class="action-icon">${a.icon}</span>
          <span class="action-title">${a.title}</span>
          <span class="action-when">${a.when}</span>
        </div>
        <p class="action-detail">${a.detail}</p>
      </li>
    `
    )
    .join('');

  const notesList = $('notes-list');
  notesList.innerHTML = out.notes.map((n) => `<li>${n}</li>`).join('');

  // 台帳の検証表示は「やる気スイッチ」ドロワーが開いているときのみ更新（普段はDOM再構築コスト0）
  const proDrawer = $                    ('drawer-ledger-pro');
  if (proDrawer && proDrawer.open) {
    renderLedgerView();
  }
}

function renderDependentRow(yr                 , y            )       {
  const taxBadge = $(`dep-tax-badge-${yr}`);
  taxBadge.className = `status-badge ${statusBadgeClass(y.taxDependent.status)}`;
  taxBadge.textContent = `${statusIcon(y.taxDependent.status)} ${y.taxDependent.label}`;
  $(`dep-tax-msg-${yr}`).textContent = y.taxDependent.message;

  const insBadge = $(`dep-ins-badge-${yr}`);
  insBadge.className = `status-badge ${statusBadgeClass(y.insuranceDependent.status)}`;
  insBadge.textContent = `${statusIcon(y.insuranceDependent.status)} ${y.insuranceDependent.label}`;
  $(`dep-ins-msg-${yr}`).textContent = y.insuranceDependent.message;
}

function renderTaxBreakdown(yr                 , y            )       {
  $(`bd-total-${yr}`).textContent = fmtMan(y.total);
  $(`bd-total-exact-${yr}`).textContent = fmtExactYen(y.total);
  $(`bd-it-${yr}`).textContent = fmtMan(y.incomeTax);
  $(`bd-rt-${yr}`).textContent = fmtMan(y.residentTax);
  $(`bd-nhi-${yr}`).textContent = y.nhi > 0 ? fmtMan(y.nhi) : '0円（扶養内）';
  $(`bd-pen-${yr}`).textContent =
    y.pension > 0 ? fmtMan(y.pension) : '0円（学生猶予/免除・20歳未満）';
  $(`bd-basic-${yr}`).textContent = `基礎控除 ${fmtMan(y.basicDeduction)}`;
}

/** 音声テキストをパースしてステート＋UIに反映（地域が発話されなければ全国平均を維持） */
function applyVoiceTranscript(transcript        )       {
  const transcriptBox = $                  ('voice-transcript-input');
  if (transcriptBox && transcriptBox.value !== transcript) {
    transcriptBox.value = transcript;
  }

  const parsed = parseVoiceText(transcript);

  if (parsed.age !== undefined) state.age = parsed.age;
  if (parsed.prefecture !== undefined) {
    state.prefecture = parsed.prefecture;
  }
  if (parsed.income2025 !== undefined) state.income2025 = parsed.income2025;
  if (parsed.expense2025 !== undefined) state.expense2025 = parsed.expense2025;
  if (parsed.income2026 !== undefined) state.income2026 = parsed.income2026;
  if (parsed.expense2026 !== undefined) state.expense2026 = parsed.expense2026;
  if (parsed.startMonth2025 !== undefined) state.startMonth2025 = parsed.startMonth2025;
  if (parsed.hasSalary !== undefined) state.hasSalary = parsed.hasSalary;
  if (parsed.salaryAnnual !== undefined) state.salaryAnnual = parsed.salaryAnnual;
  if (parsed.blueReturn !== undefined) state.blueReturn = parsed.blueReturn;
  if (parsed.filed2025 !== undefined) state.filed2025 = parsed.filed2025;
  if (parsed.livesAlone !== undefined) state.livesAlone = parsed.livesAlone;

  syncControlsFromState();
  // 音声・テキスト入力中はテキスト欄自体を上書きしないよう syncTranscriptText = false で描画
  recalculateAndRender(false);
}

/** Web Speech API のセットアップ（getUserMedia とのマイク競合を排除＋アプリ内ブラウザ時はOS音声ディクテーションへ直結） */
function setupVoiceRecognition()       {
  const micBtn = $                   ('mic-btn');
  const micEmoji = $('mic-emoji');
  const micLabelMain = $('mic-label-main');
  const micLabelSub = $('mic-label-sub');
  const micLevelWrap = $('mic-level-wrap');
  const micLevelBar = $('mic-level-bar');
  const micStatus = $('mic-status');
  const transcriptInput = $                  ('voice-transcript-input');

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const SpeechRecognitionCtor =
    (window       ).SpeechRecognition || (window       ).webkitSpeechRecognition;

  let recognition      = null;
  let isListening = false;
  let userWantsListening = false;
  let gotAnyResultInSession = false;
  let accumulatedFinalText = '';
  let speechApiUnavailable = !SpeechRecognitionCtor;

  // Web Speech API 自身が発火する soundstart / audiostart に合わせた疑似メーター（getUserMedia でマイクを奪わない！）
  let pulseTimer                = null;
  const startVisualMeter = () => {
    micLevelWrap.classList.remove('hidden');
    if (pulseTimer !== null) window.clearInterval(pulseTimer);
    pulseTimer = window.setInterval(() => {
      const w = 20 + Math.round(Math.random() * 65);
      micLevelBar.style.width = `${w}%`;
    }, 140);
  };
  const stopVisualMeter = () => {
    if (pulseTimer !== null) {
      window.clearInterval(pulseTimer);
      pulseTimer = null;
    }
    micLevelWrap.classList.add('hidden');
    micLevelBar.style.width = '8%';
  };

  const updateMicButtonUI = (listening         ) => {
    micBtn.classList.toggle('listening', listening);
    if (listening) {
      micEmoji.textContent = '🛑';
      micLabelMain.textContent = '録音中… 話し終わったらここをタップ';
      micLabelSub.textContent = '少し間を空けて話しても続けて聞き取ります';
    } else {
      micEmoji.textContent = '🎙️';
      micLabelMain.textContent = 'タップして状況を喋る（録音）';
      micLabelSub.textContent = '「20歳、去年10月から50万、今年は250万、経費30万くらい」';
    }
  };

  /** アプリ内ブラウザ等でクラウド音声APIが塞がれているときのOS標準ディクテーション・モード */
  const activateOsDictationMode = (reason        ) => {
    userWantsListening = false;
    isListening = false;
    transcriptInput.value = '';
    transcriptInput.placeholder =
      '🎙️ ここが選択されています！Macは「Fnキーを2回押す」、スマホはキーボードの「🎤」でそのまま喋ってください';
    transcriptInput.focus();
    micBtn.classList.add('listening');
    micEmoji.textContent = '⌨️🎙️';
    micLabelMain.textContent = '下の入力欄にマイクをセットしました！';
    micLabelSub.textContent =
      'Macはキーボード左下の「Fn（地球儀）キー2回押し」、スマホはキーボードの「🎤」を押して喋ると即反映されます';
    micStatus.innerHTML = `💡 お使いの埋め込み画面（${reason}）ではOS標準の音声入力を使います。<strong>今すぐキーボードの「Fnキー2回押し（または🎤ボタン）」で喋ってみてください！</strong>`;
  };

  if (SpeechRecognitionCtor) {
    recognition = new SpeechRecognitionCtor();
    recognition.lang = 'ja-JP';
    recognition.interimResults = true;
    recognition.continuous = true;
    recognition.maxAlternatives = 1;

    recognition.onstart = () => {
      isListening = true;
      updateMicButtonUI(true);
      startVisualMeter();
      micStatus.innerHTML =
        '🔴 <strong>マイク聞き取り中！</strong>そのまま「20歳、去年50万、今年250万、経費30万」のように話してください';
    };

    recognition.onresult = (event     ) => {
      gotAnyResultInSession = true;
      let interimText = '';
      for (let i = event.resultIndex; i < event.results.length; i++) {
        const chunk = event.results[i][0].transcript;
        if (event.results[i].isFinal) {
          accumulatedFinalText += chunk + '、';
        } else {
          interimText += chunk;
        }
      }
      const fullText = (accumulatedFinalText + interimText).replace(/、+$/g, '');
      if (fullText.trim()) {
        transcriptInput.value = fullText;
        applyVoiceTranscript(fullText);
        micStatus.innerHTML = `🎙️ 聞き取り中: <strong>「${fullText}」</strong>（話し終わったら上の赤いボタンをタップ）`;
      }
    };

    recognition.onerror = (event     ) => {
      const err = String(event.error || '');
      if (err === 'no-speech' && userWantsListening) {
        micStatus.innerHTML =
          '🔴 <strong>声を待っています…</strong>マイクに向かって「20歳、去年50万、今年250万」のように話してください';
        return;
      }

      stopVisualMeter();
      userWantsListening = false;
      isListening = false;
      updateMicButtonUI(false);

      if (err === 'network' || err === 'service-not-allowed' || err === 'aborted') {
        // Electron / 内蔵WebView で Google Speech API が無効な場合
        speechApiUnavailable = true;
        activateOsDictationMode(`内蔵ブラウザ制限: ${err}`);
      } else if (err === 'not-allowed') {
        micStatus.innerHTML =
          '⚠️ マイク権限が許可されていません。アドレスバー左のアイコンからマイクを「許可」してもう一度お試しください。';
      } else {
        activateOsDictationMode(err || '音声エンジン切替');
      }
    };

    recognition.onend = () => {
      isListening = false;
      if (userWantsListening && !gotAnyResultInSession && !speechApiUnavailable) {
        try {
          recognition.start();
          return;
        } catch {
          // ignore
        }
      }
      stopVisualMeter();
      if (!speechApiUnavailable) {
        userWantsListening = false;
        updateMicButtonUI(false);
        if (gotAnyResultInSession) {
          micStatus.innerHTML =
            '✨ <strong>あなたの声から自動セット完了！</strong>すぐ下の診断結果と青色/白色ボタンをチェックしてください';
        }
      }
    };
  }

  micBtn.addEventListener('click', () => {
    if (speechApiUnavailable || !recognition) {
      activateOsDictationMode('OS標準ディクテーション');
      return;
    }

    if (isListening || userWantsListening) {
      userWantsListening = false;
      stopVisualMeter();
      try {
        recognition.stop();
      } catch {
        // ignore
      }
      updateMicButtonUI(false);
      micStatus.innerHTML = '⏹️ 録音を停止しました。もう一度タップすると再開します。';
      return;
    }

    // クリックイベントの中で同期的に recognition.start() だけを呼ぶ（getUserMediaとのマイク奪い合いを防ぐ）
    userWantsListening = true;
    gotAnyResultInSession = false;
    accumulatedFinalText = '';
    transcriptInput.value = '';
    transcriptInput.placeholder = '🎙️ 聞き取り中…「20歳、去年50万、今年250万、経費30万」';

    try {
      recognition.start();
    } catch {
      try {
        recognition.stop();
      } catch {
        // ignore
      }
    }
  });

  // IME（Mac音声入力・日本語変換）確定時にも確実にパースを実行
  let isComposing = false;
  transcriptInput.addEventListener('compositionstart', () => {
    isComposing = true;
  });
  transcriptInput.addEventListener('compositionend', () => {
    isComposing = false;
    if (transcriptInput.value.trim()) {
      applyVoiceTranscript(transcriptInput.value);
      micBtn.classList.remove('listening');
      micEmoji.textContent = '🎙️';
      micLabelMain.textContent = 'タップして状況を喋る（録音）';
      micLabelSub.textContent = '「20歳、去年10月から50万、今年は250万、経費30万くらい」';
      micStatus.innerHTML =
        '✨ <strong>音声入力から自動セット完了！</strong>すぐ下のチップと診断結果に反映されました';
    }
  });

  transcriptInput.addEventListener('input', () => {
    if (isComposing) return;
    if (transcriptInput.value.trim()) {
      applyVoiceTranscript(transcriptInput.value);
      micBtn.classList.remove('listening');
      micEmoji.textContent = '🎙️';
      micLabelMain.textContent = 'タップして状況を喋る（録音）';
      micLabelSub.textContent = '「20歳、去年10月から50万、今年は250万、経費30万くらい」';
    }
  });

  document.querySelectorAll                   ('[data-voice-sample]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const sample = btn.dataset.voiceSample || '';
      applyVoiceTranscript(sample);
      micStatus.textContent = '✨ 例文から一発セットしました！すぐ下の青色/白色ボタンで比較できます';
    });
  });
}

/** フォーム・スライダー・「−/＋」ボタンのイベント登録 */
function setupControls()       {
  // 都道府県セレクト（全国平均を先頭に配置し、その下に47都道府県を並べる）
  const prefSelect = $                   ('pref-select');
  const prefs = getPrefectureList();
  const avgItem = prefs.find((p) => p.id === NATIONAL_AVERAGE_PREF);
  const otherItems = prefs.filter((p) => p.id !== NATIONAL_AVERAGE_PREF);
  const ordered = avgItem ? [avgItem, ...otherItems] : prefs;

  prefSelect.innerHTML = ordered
    .map((p) =>
      p.id === NATIONAL_AVERAGE_PREF
        ? `<option value="${p.id}">🛡️ 地域未指定：全国平均で計算（身バレ防止・推奨初期値）</option>`
        : `<option value="${p.id}">📍 ${p.name}（代表基準：${p.city}）</option>`
    )
    .join('');
  prefSelect.value = String(state.prefecture);
  prefSelect.addEventListener('change', () => {
    state.prefecture = parseInt(prefSelect.value, 10);
    recalculateAndRender();
  });

  // 年齢区分ボタン
  document.querySelectorAll                   ('[data-age-group]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const g = btn.dataset.ageGroup;
      state.age = g === 'u18' ? 17 : g === 'student' ? 20 : 25;
      syncControlsFromState();
      recalculateAndRender();
    });
  });

  // 本業・バイト有無ボタン
  document.querySelectorAll                   ('[data-has-salary]').forEach((btn) => {
    btn.addEventListener('click', () => {
      state.hasSalary = btn.dataset.hasSalary === 'true';
      if (state.hasSalary && state.salaryAnnual === 0) {
        state.salaryAnnual = 80 * MAN;
      }
      syncControlsFromState();
      recalculateAndRender();
    });
  });

  const salSlider = $                  ('slider-salary');
  salSlider.addEventListener('input', () => {
    state.salaryAnnual = parseInt(salSlider.value, 10) * MAN;
    $('val-salary').textContent = `${salSlider.value}万円`;
    recalculateAndRender();
  });

  // 事業者登録（青色 / 白色）色分け一発変換ボタン
  document.querySelectorAll                   ('[data-blue-return]').forEach((btn) => {
    btn.addEventListener('click', () => {
      state.blueReturn = btn.dataset.blueReturn === 'true';
      syncControlsFromState();
      recalculateAndRender();
    });
  });

  // 2025年（昨年）スライダー＆クイックチップ
  const inc25Slider = $                  ('slider-inc-2025');
  const exp25Slider = $                  ('slider-exp-2025');
  inc25Slider.addEventListener('input', () => {
    state.income2025 = parseInt(inc25Slider.value, 10) * MAN;
    $('val-inc-2025').textContent = `${inc25Slider.value}万円`;
    recalculateAndRender();
  });
  exp25Slider.addEventListener('input', () => {
    state.expense2025 = parseInt(exp25Slider.value, 10) * MAN;
    $('val-exp-2025').textContent = `${exp25Slider.value}万円`;
    recalculateAndRender();
  });

  document.querySelectorAll                   ('[data-quick-2025]').forEach((btn) => {
    btn.addEventListener('click', () => {
      state.income2025 = parseInt(btn.dataset.quick2025 || '0', 10) * MAN;
      syncControlsFromState();
      recalculateAndRender();
    });
  });

  const startSelect = $                   ('select-start-2025');
  startSelect.addEventListener('change', () => {
    state.startMonth2025 = parseInt(startSelect.value, 10) || 0;
    recalculateAndRender();
  });

  const filed25Check = $                  ('check-filed-2025');
  filed25Check.addEventListener('change', () => {
    state.filed2025 = filed25Check.checked;
    recalculateAndRender();
  });

  // 2026年（今年）スライダー＆クイックチップ
  const inc26Slider = $                  ('slider-inc-2026');
  const exp26Slider = $                  ('slider-exp-2026');
  inc26Slider.addEventListener('input', () => {
    state.income2026 = parseInt(inc26Slider.value, 10) * MAN;
    $('val-inc-2026').textContent = `${inc26Slider.value}万円`;
    recalculateAndRender();
  });
  exp26Slider.addEventListener('input', () => {
    state.expense2026 = parseInt(exp26Slider.value, 10) * MAN;
    $('val-exp-2026').textContent = `${exp26Slider.value}万円`;
    recalculateAndRender();
  });

  document.querySelectorAll                   ('[data-quick-2026]').forEach((btn) => {
    btn.addEventListener('click', () => {
      state.income2026 = parseInt(btn.dataset.quick2026 || '0', 10) * MAN;
      syncControlsFromState();
      recalculateAndRender();
    });
  });

  // すべてのスライダー横の「− / ＋」ステップボタン
  document.querySelectorAll                   ('[data-step-target]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const targetId = btn.dataset.stepTarget;
      const delta = parseInt(btn.dataset.stepDelta || '0', 10);
      if (!targetId) return;
      const slider = $                  (targetId);
      if (!slider) return;
      const min = parseInt(slider.min || '0', 10);
      const max = parseInt(slider.max || '1000', 10);
      const cur = parseInt(slider.value || '0', 10);
      const next = Math.min(max, Math.max(min, cur + delta));
      slider.value = String(next);
      slider.dispatchEvent(new Event('input', { bubbles: true }));
    });
  });

  // LINE共有・テキストコピー・スクショモード（DOM追加/削除やGPUブラーを使わず超軽量動作）
  const copyBtn = $                   ('btn-copy-line');
  const shareModal = $('share-modal');
  const shareTextarea = $                     ('share-modal-textarea');
  const shareStatus = $('share-modal-status');

  const closeShareModal = () => {
    shareModal.classList.add('hidden');
  };

  $('btn-modal-close-bottom').addEventListener('click', closeShareModal);
  $('share-modal-backdrop').addEventListener('click', closeShareModal);

  /** モーダル内の既存の textarea を使ってゼロDOMアロケーションで同期コピー */
  const copyFromModalTextarea = ()          => {
    try {
      shareTextarea.focus({ preventScroll: true });
      shareTextarea.select();
      shareTextarea.setSelectionRange(0, shareTextarea.value.length);
      return document.execCommand('copy');
    } catch {
      return false;
    }
  };

  copyBtn.addEventListener('click', () => {
    const out = runDiagnose(state);
    const [y25, y26] = out.years;
    const monthlyReserve = Math.ceil(out.lockTotal / 12 / 100) * 100;
    const rtOnce = y26.residentTax > 0 ? Math.ceil(y26.residentTax / 4 / 100) * 100 : 0;
    const nhiOnce = y26.nhi > 0 ? Math.ceil(y26.nhi / 10 / 100) * 100 : 0;
    const shareText = [
      '【TikTokライバー 税金・国保・親の扶養 一撃診断】',
      out.headline,
      `📍 ${out.prefectureName} / ${state.age}歳 / ${state.blueReturn ? '青色申告(65万控除)' : '白色(未登録)'}`,
      '',
      `■ 2026年（今年）振込${fmtMan(y26.gross)}・経費${fmtMan(y26.expense)} → 所得${fmtMan(y26.businessIncome)}`,
      `・税金の扶養: ${y26.taxDependent.label} / 健康保険証: ${y26.insuranceDependent.label}`,
      '',
      `📮 役所から届く納付書の分割目安（月ならし 約${fmtMan(monthlyReserve)}）`,
      `・3月（所得税・年1回）: 約${fmtMan(y26.incomeTax)}`,
      `・6月〜（住民税・年4回）: 1回 約${fmtMan(rtOnce)}`,
      `・6月〜（国保・年10回）: 1回 約${fmtMan(nhiOnce)}`,
      `（年間の現実ライン合計: 約${fmtMan(out.lockTotal)} / 2025年分 約${fmtMan(y25.total)}含む）`,
      '',
      `💰 手元に残る安全なお金: 約${fmtMan(out.freeMoney)}`,
    ].join('\n');

    shareTextarea.value = shareText;
    shareModal.classList.remove('hidden');

    const copiedSync = copyFromModalTextarea();
    if (copiedSync) {
      shareStatus.textContent = '✅ そのままLINEやメモ帳に貼り付け（ペースト）できます。';
    } else {
      shareStatus.textContent = '📋 下の「コピー」ボタンを押してLINEやメモ帳に貼り付けてください。';
    }
  });

  const modalCopyBtn = $                   ('btn-modal-copy-again');
  modalCopyBtn.addEventListener('click', () => {
    const ok = copyFromModalTextarea();
    if (ok) {
      shareStatus.textContent = '✅ そのままLINEやメモ帳に貼り付け（ペースト）できます。';
      modalCopyBtn.textContent = '✅ コピー済';
      setTimeout(() => {
        modalCopyBtn.textContent = '📋 コピー';
      }, 1800);
    } else {
      shareStatus.textContent = '📋 テキストを選択しました（Command+C または長押しでコピー）。';
    }
  });

  // すべての折りたたみドロワー（<details>）で開閉時に「タップで開く ▼」⇔「タップで閉じる ▲」を自動切替
  document.querySelectorAll                    ('details').forEach((det) => {
    det.addEventListener('toggle', () => {
      const hint = det.querySelector             ('[data-open-text]');
      if (hint) {
        const openTxt = hint.dataset.openText || 'タップで開く ▼';
        const closeTxt = hint.dataset.closeText || 'タップで閉じる ▲';
        hint.textContent = det.open ? closeTxt : openTxt;
      }
    });
  });

  const shotBtn = $                   ('btn-screenshot-mode');
  shotBtn.addEventListener('click', () => {
    document.body.classList.toggle('screenshot-view');
    const isShot = document.body.classList.contains('screenshot-view');
    shotBtn.textContent = isShot ? '✏️ 入力・台帳表示に戻す' : '📸 スクショ1画面';
    if (isShot) {
      $('result-card').scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  });
}

/** STEP 4: 確定申告・税金管理台帳の初期化と描画 */
function setupLedger()       {
  ledgerEntries = loadLedger();

  const typeInput = $                  ('lf-type');
  const catSelect = $                   ('lf-category');
  const yearSelect = $                   ('lf-year');
  const monthSelect = $                   ('lf-month');
  const daySelect = $                   ('lf-day');
  const amtInput = $                  ('lf-amount');
  const memoInput = $                  ('lf-memo');
  const submitFeedback = $('lf-submit-feedback');

  // 各項目の「❓」ボタンを押したときに、その項目の真下に開く吹き出し解説辞書
  const HELP_MESSAGES                                                  = {
    type: {
      title: '💡 ①「売上」と「経費」の切り替え方',
      text: 'TikTokから口座に振り込まれた報酬や案件ギャラを記録するときは左の「💴 入ってきたお金（売上）」、配信のために買ったライト・マイク・衣装・コスメ・スマホ通信費などを記録するときは右の「🧾 配信に使ったお金（経費）」をタップします。',
    },
    date: {
      title: '💡 ②「日付」の選び方（月まとめでもOK）',
      text: '「年・月・日」を選ぶか、下の「📅 今月末（月まとめ）」ボタンをポンと押すだけでセットできます。毎日の細かい日付がわからなくても、月末の日付で「11月分まとめ」として記帳して確定申告の年額計算はバッチリ合います！',
    },
    category: {
      title: '💡 ③「種類（勘定科目）」の自動集計について',
      text: '1件ずつ「➕ 台帳に記帳する」を押すたびに、同じ種類（消耗品費・衣装美容費・通信費など）が自動でグループ分けされて、すぐ下の「📂 確定申告用・経費の勘定科目別集計」の表にリアルタイムで足し算されます。',
    },
    amount: {
      title: '💡 ④「金額」の入力方法（1円単位でも万単位でもOK）',
      text: '①で「売上」を選んでいるときは「口座に振り込まれた金額（例: 158420）」、「経費」を選んでいるときは「レシートや明細の支払金額（例: 3985）」を入力します。1円単位の端数でも、下の「＋1万円」ボタンでも自由に入力できます！',
    },
    memo: {
      title: '💡 ⑤「メモ」の書き方（空欄でも自動入力）',
      text: '自分があとで見返したときに「11月分のTikTok振込」「Amazonで買ったライト」とわかる一言でOKです。空欄のまま記帳ボタンを押しても、③で選んだ種類名が自動で入るので安心です！',
    },
  };

  let openHelpKey                = null;

  const toggleInlineHelp = (key        , forceOpen = false) => {
    const targetPop = document.getElementById(`lf-pop-${key}`);
    if (!targetPop) return;

    const shouldClose = !forceOpen && openHelpKey === key && !targetPop.classList.contains('hidden');

    // まず全ての吹き出しとボタンの選択状態をリセット
    Object.keys(HELP_MESSAGES).forEach((k) => {
      const pop = document.getElementById(`lf-pop-${k}`);
      if (pop) pop.classList.add('hidden');
    });
    document.querySelectorAll                   ('[data-help-key]').forEach((b) => {
      b.classList.remove('active');
    });

    if (shouldClose) {
      openHelpKey = null;
      return;
    }

    const info = HELP_MESSAGES[key];
    if (!info) return;
    targetPop.innerHTML = `
      <div class="lf-pop-head">
        <strong>${info.title}</strong>
        <button type="button" class="lf-pop-close" data-close-pop="${key}">✕ 閉じる</button>
      </div>
      <p>${info.text}</p>
    `;
    targetPop.classList.remove('hidden');
    const chip = document.querySelector                   (`[data-help-key="${key}"]`);
    if (chip) chip.classList.add('active');
    openHelpKey = key;

    const closeBtn = targetPop.querySelector                   (`[data-close-pop="${key}"]`);
    if (closeBtn) {
      closeBtn.addEventListener('click', () => toggleInlineHelp(key, false));
    }
  };

  // 「❓」チップをクリックしたときだけ、その真下に吹き出しを開閉する
  document.querySelectorAll                   ('[data-help-key]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const k = btn.dataset.helpKey;
      if (k) toggleInlineHelp(k, false);
    });
  });

  amtInput.addEventListener('focus', () => {
    amtInput.classList.remove('input-error-shake');
    submitFeedback.classList.add('hidden');
  });

  // 年・月・日のプルダウン初期化（月末日数を自動調整）
  monthSelect.innerHTML = Array.from(
    { length: 12 },
    (_, i) => `<option value="${i + 1}">${i + 1}月</option>`
  ).join('');

  const updateDaysInMonth = () => {
    const y = parseInt(yearSelect.value, 10) || 2026;
    const m = parseInt(monthSelect.value, 10) || 11;
    const maxDay = new Date(y, m, 0).getDate();
    const prevDay = parseInt(daySelect.value, 10) || 16;
    daySelect.innerHTML = Array.from(
      { length: maxDay },
      (_, i) => `<option value="${i + 1}">${i + 1}日</option>`
    ).join('');
    daySelect.value = String(Math.min(maxDay, prevDay));
  };

  yearSelect.value = '2026';
  monthSelect.value = '11';
  updateDaysInMonth();
  daySelect.value = '16';

  yearSelect.addEventListener('change', updateDaysInMonth);
  monthSelect.addEventListener('change', updateDaysInMonth);

  // ワンタップ日付プリセットボタン
  document.querySelectorAll                   ('[data-date-preset]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const preset = btn.dataset.datePreset;
      const now = new Date();
      if (preset === 'today') {
        const y = now.getFullYear() === 2025 ? 2025 : 2026;
        yearSelect.value = String(y);
        monthSelect.value = String(now.getMonth() + 1);
        updateDaysInMonth();
        daySelect.value = String(now.getDate());
      } else if (preset === 'this-month-end') {
        const y = parseInt(yearSelect.value, 10) || 2026;
        const m = parseInt(monthSelect.value, 10) || 11;
        const maxDay = new Date(y, m, 0).getDate();
        updateDaysInMonth();
        daySelect.value = String(maxDay);
      } else if (preset === 'last-month-end') {
        let y = parseInt(yearSelect.value, 10) || 2026;
        let m = (parseInt(monthSelect.value, 10) || 11) - 1;
        if (m < 1) {
          m = 12;
          y = 2025;
        }
        yearSelect.value = String(y);
        monthSelect.value = String(m);
        updateDaysInMonth();
        daySelect.value = String(new Date(y, m, 0).getDate());
      } else if (preset === '2025-end') {
        yearSelect.value = '2025';
        monthSelect.value = '12';
        updateDaysInMonth();
        daySelect.value = '31';
      }
    });
  });

  // 売上 / 経費の切替に応じてラベル・プレースホルダー・勘定科目リストを動的に切替
  const syncLedgerTypeUI = (newType                 ) => {
    typeInput.value = newType;
    const isIncome = newType === 'income';
    $('btn-lf-income').classList.toggle('active', isIncome);
    $('btn-lf-expense').classList.toggle('active', !isIncome);

    const cats = isIncome ? INCOME_CATEGORIES : EXPENSE_CATEGORIES;
    catSelect.innerHTML = cats.map((c) => `<option value="${c}">${c}</option>`).join('');

    $('lf-date-label').textContent = isIncome
      ? 'いつの振込・売上ですか？（振込日または月末まとめ）'
      : 'いつ支払った経費ですか？（レシートの日付または月末まとめ）';
    $('lf-cat-label').textContent = isIncome
      ? '売上の種類（確定申告の収入項目に自動分類）'
      : '使った経費の種類（確定申告の経費科目に自動分類）';
    $('lf-amount-label').textContent = isIncome
      ? '振り込まれた金額（1円単位で自由に入力OK）'
      : '支払った経費の金額（1円単位・レシート通りでOK）';
    amtInput.placeholder = isIncome
      ? '例: 158420（振り込まれた金額）'
      : '例: 4980（ライト・衣装・通信費などの支払額）';
    memoInput.placeholder = isIncome
      ? '例: 11月分 TikTok振込（空欄なら種類名が自動で入ります）'
      : '例: 配信用リングライト購入（空欄なら種類名が自動で入ります）';
  };

  syncLedgerTypeUI('income');

  document.querySelectorAll                   ('[data-lf-type]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const t = btn.dataset.lfType === 'expense' ? 'expense' : 'income';
      syncLedgerTypeUI(t);
    });
  });

  // 金額のワンタップ加算ボタン（＋1千円 / ＋1万円 / ＋5万円 / ＋10万円 / クリア）
  document.querySelectorAll                   ('[data-amt-add]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const add = parseInt(btn.dataset.amtAdd || '0', 10);
      const cur = Math.max(0, parseInt(amtInput.value || '0', 10) || 0);
      amtInput.value = String(cur + add);
      amtInput.classList.remove('input-error-shake');
      submitFeedback.classList.add('hidden');
    });
  });
  document.querySelectorAll                   ('[data-amt-clear]').forEach((btn) => {
    btn.addEventListener('click', () => {
      amtInput.value = '';
    });
  });

  // 仕訳追加フォーム送信（空欄時は金額欄の吹き出し＋ボタン直上のガイドを表示）
  const form = $                 ('ledger-form');
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    const y = yearSelect.value || '2026';
    const m = String(parseInt(monthSelect.value || '11', 10)).padStart(2, '0');
    const d = String(parseInt(daySelect.value || '16', 10)).padStart(2, '0');
    const dateVal = `${y}-${m}-${d}`;

    const typeVal = (typeInput.value === 'expense' ? 'expense' : 'income')                   ;
    const catVal = catSelect.value                   ;
    const amount = Math.floor(Number(amtInput.value));

    if (!amtInput.value || Number.isNaN(amount) || amount <= 0) {
      amtInput.classList.add('input-error-shake');
      amtInput.focus();
      toggleInlineHelp('amount', true);
      submitFeedback.className = 'lf-submit-feedback is-error';
      submitFeedback.innerHTML = `
        <strong>⚠️ ④「金額」を入力してから記帳ボタンを押してください！</strong>
        <span>${
          typeVal === 'income'
            ? '口座に振り込まれた金額（例: 158420 など1円単位でもOK）を入力するか、上の「＋1万円」「＋5万円」ボタンを押してください。'
            : '支払った経費の金額（例: 3985 などレシート通り1円単位でOK）を入力するか、上の「＋1千円」「＋1万円」ボタンを押してください。'
        }</span>
      `;
      return;
    }

    amtInput.classList.remove('input-error-shake');

    const newEntry              = {
      id: `entry-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      date: dateVal,
      type: typeVal,
      category: catVal,
      amount,
      memo: memoInput.value.trim() || catVal,
    };

    ledgerEntries.push(newEntry);
    saveLedger(ledgerEntries);

    // 追加した日付の年度タブに自動切替
    activeLedgerYear = y === '2025' ? 2025 : 2026;
    syncLedgerYearTabs();
    renderLedgerView();

    submitFeedback.className = 'lf-submit-feedback is-success';
    submitFeedback.innerHTML = `
      <strong>✅ ${dateVal}「${catVal}」${fmtExactYen(amount)} を記帳しました！</strong>
      <span>下の「勘定科目別集計」「月別一覧」「仕訳帳」に自動反映されました。</span>
    `;

    amtInput.value = '';
    memoInput.value = '';
  });

  // 台帳の集計値で上の診断スライダー＆結果を更新するボタン
  const syncBtn = $                   ('btn-sync-ledger-to-diag');
  syncBtn.addEventListener('click', () => {
    const sum25 = summarizeLedgerByYear(ledgerEntries, 2025, state.blueReturn);
    const sum26 = summarizeLedgerByYear(ledgerEntries, 2026, state.blueReturn);
    state.income2025 = sum25.totalIncome;
    state.expense2025 = sum25.totalExpense;
    state.income2026 = sum26.totalIncome;
    state.expense2026 = sum26.totalExpense;
    syncControlsFromState();
    recalculateAndRender();
    $('result-card').scrollIntoView({ behavior: 'smooth', block: 'start' });
  });

  // 年度タブ切替（2026 / 2025）
  document.querySelectorAll                   ('[data-ledger-year]').forEach((btn) => {
    btn.addEventListener('click', () => {
      activeLedgerYear = btn.dataset.ledgerYear === '2025' ? 2025 : 2026;
      syncLedgerYearTabs();
      renderLedgerView();
    });
  });

  // CSV出力ボタン
  const csvBtn = $                   ('btn-export-csv');
  csvBtn.addEventListener('click', () => {
    const csv = exportLedgerCsv(ledgerEntries);
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `tiktok_liver_tax_ledger_${activeLedgerYear}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  });

  // やる気スイッチ（台帳ドロワー）が開かれたときに初めて台帳テーブルを描画
  const proDrawer = $                    ('drawer-ledger-pro');
  proDrawer.addEventListener('toggle', () => {
    if (proDrawer.open) {
      renderLedgerView();
    }
  });
}

function syncLedgerYearTabs()       {
  document.querySelectorAll                   ('[data-ledger-year]').forEach((btn) => {
    const yr = btn.dataset.ledgerYear === '2025' ? 2025 : 2026;
    btn.classList.toggle('active', yr === activeLedgerYear);
  });
}

function renderLedgerView()       {
  const sum = summarizeLedgerByYear(ledgerEntries, activeLedgerYear, state.blueReturn);

  $('lv-total-inc').textContent = fmtExactYen(sum.totalIncome);
  $('lv-total-exp').textContent = fmtExactYen(sum.totalExpense);
  $('lv-net-profit').textContent = fmtExactYen(sum.netProfit);

  // 検証チェックリスト
  $('lv-check-list').innerHTML = sum.checks
    .map((c) => `<li>${c.ok ? '✅' : '⚠️'} ${c.text}</li>`)
    .join('');

  // 勘定科目別テーブル
  const catTbody = $('ledger-cat-tbody');
  if (sum.byCategory.length === 0) {
    catTbody.innerHTML = '<tr><td colspan="2">この年度の経費データはまだありません</td></tr>';
  } else {
    catTbody.innerHTML = sum.byCategory
      .map((item) => `<tr><td>${item.category}</td><td class="td-expense">${fmtExactYen(item.amount)}</td></tr>`)
      .join('');
  }

  // 月別推移テーブル（売上または経費がある月、なければ全月）
  const monthTbody = $('ledger-month-tbody');
  const activeMonths = sum.monthly.filter((m) => m.income > 0 || m.expense > 0);
  const rowsToShow = activeMonths.length > 0 ? activeMonths : sum.monthly;
  monthTbody.innerHTML = rowsToShow
    .map((m) => {
      const diff = m.income - m.expense;
      return `<tr>
        <td>${m.month}月</td>
        <td class="td-income">${m.income > 0 ? fmtExactYen(m.income) : '-'}</td>
        <td class="td-expense">${m.expense > 0 ? fmtExactYen(m.expense) : '-'}</td>
        <td>${diff !== 0 ? fmtExactYen(diff) : '-'}</td>
      </tr>`;
    })
    .join('');

  // 仕訳帳（日付順一覧）
  const entriesTbody = $('ledger-entries-tbody');
  const yearEntries = ledgerEntries
    .filter((e) => e.date.startsWith(`${activeLedgerYear}-`))
    .sort((a, b) => a.date.localeCompare(b.date));

  if (yearEntries.length === 0) {
    entriesTbody.innerHTML =
      '<tr><td colspan="6">まだ記帳データがありません。上のフォームから日付と金額を追加してください。</td></tr>';
  } else {
    entriesTbody.innerHTML = yearEntries
      .map(
        (e) => `<tr>
        <td>${e.date}</td>
        <td class="${e.type === 'income' ? 'td-income' : 'td-expense'}">${
          e.type === 'income' ? '売上' : '経費'
        }</td>
        <td>${e.category}</td>
        <td class="${e.type === 'income' ? 'td-income' : 'td-expense'}">${fmtExactYen(e.amount)}</td>
        <td>${e.memo}</td>
        <td><button type="button" class="btn-del-entry" data-del-id="${e.id}">削除</button></td>
      </tr>`
      )
      .join('');

    entriesTbody.querySelectorAll                   ('[data-del-id]').forEach((btn) => {
      btn.addEventListener('click', () => {
        const id = btn.dataset.delId;
        ledgerEntries = ledgerEntries.filter((item) => item.id !== id);
        saveLedger(ledgerEntries);
        renderLedgerView();
      });
    });
  }
}

let booted = false;

/** 初期化エントリーポイント */
async function bootstrap()                {
  if (booted) return;
  booted = true;
  await initTaxEngine();
  const mode = getEngineMode();
  const engineBadge = $('engine-mode-badge');
  if (engineBadge) {
    engineBadge.textContent =
      mode === 'wasm'
        ? '🦀 Rust + WebAssembly エンジン稼働中（完全端末内計算）'
        : '🦀 Rust互換クライアント内エンジン稼働中（完全端末内計算）';
  }

  setupControls();
  setupLedger();
  setupVoiceRecognition();

  // デフォルトの発話例（身バレ防止のため地域を含まない発話例）を初期セットして診断カードを表示
  applyVoiceTranscript(
    '20歳で、去年10月から始めて50万、今年は250万振り込まれて、経費は30万くらい'
  );
}

if (typeof document !== 'undefined') {
  document.addEventListener('DOMContentLoaded', () => {
    void bootstrap();
  });
  if (document.readyState !== 'loading') {
    void bootstrap();
  }
}


//# sourceURL=main.ts