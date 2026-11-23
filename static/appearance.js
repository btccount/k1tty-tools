"use strict";
(function () {
  const KEY = "k1tty_appearance_v1";
  const defaults = Object.freeze({
    theme: "graphite",
    accent: "blue",
    surfaceTint: "",
    ambientGradient: true,
    primaryStyle: "neutral",
    customAccent: "#ed747e",
    customColors: Object.freeze({}),
    density: "comfortable",
    textSize: 14,
    lineHeight: 1.6,
    readingWidth: "normal",
    controlRadius: 8,
    panelRadius: 12,
    motion: "system",
    snow: false,
    cat: false,
    showTokenCounter: true,
    showSlimTool: true,
  });
  const classicThemes = ["graphite", "charcoal", "light", "crimson"];
  const catThemes = ["neko_night", "neko_sakura", "neko_matcha", "neko_cream"];
  const manulThemes = ["manul_steppe", "manul_snow", "manul_altai", "manul_dune"];
  const themeAccents = {
    crimson: "red",
    neko_night: "blue",
    neko_sakura: "rose",
    neko_matcha: "green",
    neko_cream: "orange",
    manul_steppe: "amber",
    manul_snow: "teal",
    manul_altai: "green",
    manul_dune: "amber",
  };
  const tintPresets = [
    { key: "", color: "", label: "tintNone" },
    { key: "#4a78c8", color: "#4a78c8", label: "blue" },
    { key: "#3d9470", color: "#3d9470", label: "green" },
    { key: "#b8823c", color: "#b8823c", label: "amber" },
    { key: "#7f63c7", color: "#7f63c7", label: "violet" },
    { key: "#be4e5e", color: "#be4e5e", label: "red" },
    { key: "#bd6d42", color: "#bd6d42", label: "orange" },
    { key: "#3d9196", color: "#3d9196", label: "teal" },
    { key: "#b86492", color: "#b86492", label: "rose" },
  ];
  const palettes = {
    graphite: { sidebar: "#101011", canvas: "#141415", surface: "#19191b", hover: "#222225", raised: "#27272a", inset: "#111112", text: "#ededed", secondary: "#b0b0b6", muted: "#919198", border: "#2c2c2e", strong: "#3d3d3f", user: "#1d1d20" },
    charcoal: { sidebar: "#1b1b1d", canvas: "#202022", surface: "#262629", hover: "#303034", raised: "#38383c", inset: "#1d1d1f", text: "#f0f0f2", secondary: "#bdbdc2", muted: "#a8a8b0", border: "#3a3a3d", strong: "#4c4c50", user: "#2c2c30" },
    light: { sidebar: "#f1f1f2", canvas: "#f8f8f9", surface: "#ffffff", hover: "#eaeaec", raised: "#e4e4e7", inset: "#f2f2f4", text: "#202024", secondary: "#55555e", muted: "#63636c", border: "#e0e0e3", strong: "#c8c8cf", user: "#f0f0f3" },
    crimson: { sidebar: "#180b0e", canvas: "#200e12", surface: "#2a141a", hover: "#381d24", raised: "#43232c", inset: "#1b0b10", text: "#f7e9ed", secondary: "#d2b7c0", muted: "#bc9aa6", border: "#4d2934", strong: "#6c3a49", user: "#341921" },
    neko_night: { sidebar: "#0d1018", canvas: "#111520", surface: "#171c2a", hover: "#202638", raised: "#252c40", inset: "#0e121b", text: "#edf0f7", secondary: "#b2bacf", muted: "#8a94b0", border: "#272f45", strong: "#3a4563", user: "#1b2132" },
    neko_sakura: { sidebar: "#161015", canvas: "#1b141a", surface: "#231a22", hover: "#2e222d", raised: "#352834", inset: "#171116", text: "#f6edf2", secondary: "#ccb8c4", muted: "#a991a0", border: "#3c2d3a", strong: "#544052", user: "#281e27" },
    neko_matcha: { sidebar: "#101512", canvas: "#141a16", surface: "#1b231e", hover: "#242f28", raised: "#2b372f", inset: "#111613", text: "#edf4ef", secondary: "#b4c5b9", muted: "#8ea395", border: "#2b3830", strong: "#405247", user: "#202923" },
    neko_cream: { sidebar: "#efeae2", canvas: "#f7f3ec", surface: "#fffdf9", hover: "#eae3d8", raised: "#e4dccf", inset: "#f1ebe2", text: "#26211c", secondary: "#5d5348", muted: "#75695b", border: "#dfd6c9", strong: "#c9bdaa", user: "#f2ece2" },
    manul_steppe: { sidebar: "#12100d", canvas: "#171410", surface: "#201c16", hover: "#2b251d", raised: "#322b22", inset: "#13110e", text: "#f3eee6", secondary: "#c8bfb2", muted: "#a39889", border: "#362f26", strong: "#4d4336", user: "#25201a" },
    manul_snow: { sidebar: "#0c1117", canvas: "#10161e", surface: "#161e29", hover: "#1f2a38", raised: "#253242", inset: "#0d1219", text: "#ecf2f8", secondary: "#b2c1d1", muted: "#899bb0", border: "#263344", strong: "#3a4d66", user: "#1a2330" },
    manul_altai: { sidebar: "#0d1210", canvas: "#111815", surface: "#18211d", hover: "#212d28", raised: "#283630", inset: "#0f1512", text: "#edf4f0", secondary: "#b4c5bc", muted: "#8ea398", border: "#293831", strong: "#3e5449", user: "#1c2722" },
    manul_dune: { sidebar: "#eee9df", canvas: "#f6f2e9", surface: "#fffdf8", hover: "#e9e2d4", raised: "#e1d8c8", inset: "#f0ebe1", text: "#25211a", secondary: "#5b5244", muted: "#746858", border: "#ded5c5", strong: "#c8bca7", user: "#f2ece1" },
  };
  const catArt = {
    neko_night: {
      tag: "neko / night",
      fx: "twinkle",
      fxChars: "SW",
      colors: { O: "#080b13", K: "#1d2336", H: "#323d5c", E: "#f6d365", P: "#d98ea4", M: "#f3d98b", S: "#89a8f5", W: "#dce6ff", L: "#2d3752", D: "#192033" },
      rows: [
        "............................",
        "...MM..............S........",
        "..MM........................",
        "..MM.......W................",
        "...MM.......................",
        "...............OO.....OO....",
        "..............OKHO...OHKO...",
        "..........S...OKKOOOOOKKO...",
        "..............OHKKKKKKKKO...",
        "..............OKEKKKKKKEO...",
        "..............OKKKKPKKKKO...",
        "..........OO...OKKKKKKKO....",
        ".........OKKO..OHKKKKKKO....",
        ".........OKO..OHKKKKKKKKO...",
        "..........OKO.OKKKHHKKKKO...",
        "...........OOOKKKHKKKHKKO...",
        "........LLLLLLLLLLLLLLLLLL..",
        "........DDDDDDDDDDDDDDDDDD..",
      ],
    },
    neko_sakura: {
      tag: "neko / sakura",
      fx: "float",
      fxChars: "FQ",
      colors: { O: "#241822", W: "#f7eef3", S: "#dcc8d4", P: "#f09ec4", B: "#f8c6de", R: "#694254", E: "#3b2434", L: "#433040", F: "#f09ec4", Q: "#f8c6de" },
      rows: [
        ".BB.........................",
        "RPPB..PB....................",
        ".RRPPBPP....................",
        "...RRRBB....................",
        ".....PB........OO.....OO....",
        "..............OWPO...OPWO...",
        "........Q.....OWWOOOOOWWO...",
        "..............OWWWWWWWWWO...",
        "..............OWEEWWWEEWO...",
        "..............OBWWWPWWWBO...",
        "....F..........OWWWWWWWO....",
        "..........OO...OWWWWWWWO....",
        ".........OWWO..OWWSSWWWO....",
        ".........OWO..OWWWSSWWWWO...",
        "..........OWO.OWWWWWWWWWO...",
        "...........OOOWWWSWWWSWWO...",
        "........LLLLLLLLLLLLLLLLLL..",
        "............................",
      ],
    },
    neko_matcha: {
      tag: "neko / matcha",
      fx: "steam",
      fxChars: "V",
      colors: { O: "#161e19", G: "#e59c5c", T: "#bd6e34", C: "#f5e8d4", P: "#e08d8d", E: "#1d2620", M: "#68b88a", D: "#458863", V: "#b9d9c2", L: "#324339" },
      rows: [
        "............................",
        "......V.....................",
        ".....V..V...................",
        "......V.....................",
        "............................",
        "..............OO.....OO.....",
        ".............OGTO...OTGO....",
        ".............OGGOOOOOGGO....",
        ".............OTGGGTGGGTO....",
        ".............OGEGGGGGEGO....",
        ".............OGGGGPGGGGO....",
        "....OOOOOO....OGGCCCGGO..OO.",
        "...OCCCCCCOO..OGCCCCGGO.OGTO",
        "...OMMMMMMOO.OGGCCCCGGGO.OGO",
        "...OMMMMMDO..OGGGCCGGGGOOGO.",
        "....OOOOOO...OGGCGGGCGGGOO..",
        "..LLLLLLLLLLLLLLLLLLLLLLLL..",
        "............................",
      ],
    },
    neko_cream: {
      tag: "neko / cream",
      fx: "sway",
      fxChars: "AG",
      colors: { O: "#40332b", W: "#fffaf2", S: "#e5d8c8", K: "#d89b62", B: "#7d5740", P: "#e09595", E: "#3d3129", A: "#6ea176", G: "#4d7c56", T: "#c97b54", L: "#d6cab9" },
      rows: [
        "............................",
        "............................",
        "............................",
        "............................",
        "............................",
        "..............OO.....OO.....",
        ".............OKPO...OPBO....",
        ".............OKKOOOOOBBO....",
        ".............OKKWWWWWWBO....",
        ".............OKEWWWWWEWO....",
        "......AA.....OWWWWPWWWWO....",
        "....AAGGAA....OWWWWWWWO..OO.",
        ".....AGGA.....OWWKKWWWO.OBBO",
        "....OOOOOO...OWWKKWWWWWO.OWO",
        "....OTTTTO...OWWWWWWWBWOOWO.",
        ".....OOOO....OWWSWWWSWWOOO..",
        "..LLLLLLLLLLLLLLLLLLLLLLLL..",
        "............................",
      ],
    },
    manul_steppe: {
      tag: "manul / steppe",
      fx: "sway",
      fxChars: "G",
      colors: { O: "#1a1510", F: "#b8aa96", L: "#ddd1be", S: "#7c6f5e", E: "#efc35b", P: "#cf9382", G: "#bfa05a", H: "#806839", R: "#504639", D: "#322b22" },
      rows: [
        "............................",
        "............................",
        "............................",
        "....G.......................",
        "...G.G.........OOOOOOO......",
        "....G.......OOOFSSSSSFOOO...",
        ".....G......OFSFFFFFFFSFO...",
        "....H.G....OLFFFEFFFEFFFLO..",
        "....H......OLLFFFPFFFFLLO...",
        "....H.......OLLLLLLLLLLO....",
        "...........OFFFLLLLLLFFFO...",
        "..........OFSFFLLLLLLFFSFO..",
        "..........OFFFLLLLLLLLFFFO..",
        "......OOOOOFSFFFFFFFFFFFSO..",
        ".....OFSFSFSFFLLLOOLLLFFFO..",
        "......OOOOOOOFFFOOOFFFOOO...",
        "..RRRRRRRRRRRRRRRRRRRRRRRR..",
        "..DDDDDDDDDDDDDDDDDDDDDDDD..",
      ],
    },
    manul_snow: {
      tag: "manul / snow",
      fx: "float",
      fxChars: "N",
      colors: { O: "#121a24", F: "#c2cedb", L: "#e6eef6", S: "#7a8da3", E: "#f0c869", P: "#cf9aa6", N: "#dce9f7", I: "#435870", D: "#263547" },
      rows: [
        "....N...............N.......",
        "............................",
        ".........N..................",
        ".......................N....",
        "..............OOOOOOO.......",
        "...........OOOFSSSSSFOOO....",
        "..N........OFSFFFFFFFSFO....",
        "..........OLFFFEFFFEFFFLO...",
        "..........OLLFFFPFFFFLLO....",
        "...........OLLLLLLLLLLO.....",
        "..........OFFFLLLLLLFFFO....",
        ".........OFSFFLLLLLLFFSFO...",
        ".........OFFFLLLLLLLLFFFO...",
        ".........OFSFFFFFFFFFFFSO...",
        "........OOLLLOOOOOOOLLLOO...",
        ".......OFSFSFSFSFSFSFSFSFO..",
        "......IIIIIIIIIIIIIIIIIIII..",
        "......DDDDDDDDDDDDDDDDDDDD..",
      ],
    },
    manul_altai: {
      tag: "manul / altai",
      fx: "sway",
      fxChars: "A",
      colors: { O: "#141c18", F: "#9e9487", L: "#d1c8ba", S: "#635c52", E: "#e6c667", P: "#c89082", A: "#5e9e74", B: "#3c6b4d", M: "#496954", R: "#3e4a43", D: "#252f29" },
      rows: [
        "............................",
        "....A.......................",
        "...AAA......................",
        "..AABAA.....................",
        "...ABA........OOOOOOO.......",
        "..AABAA....OOOFSSSSSFOOO....",
        "....B......OFSFFFFFFFSFO....",
        "....B.....OLFFFEFFFEFFFLO...",
        "....B.....OLLFFFPFFFFLLO....",
        "...........OLLLLLLLLLLO.....",
        "..........OFFFLLLLLLFFFO....",
        ".........OFSFFLLLLLLFFSFO...",
        ".........OFFFLLLLLLLLFFFO...",
        "......MM.OFSFFFFFFFFFFFSO...",
        ".....RRRRRFFLLLOOOLLLOFFO...",
        "....DRRRRRROOOFOOOFOOOOO....",
        "..RRRRRRRRRRRRRRRRRRRRRRRR..",
        "..DDDDDDDDDDDDDDDDDDDDDDDD..",
      ],
    },
    manul_dune: {
      tag: "manul / dune",
      fx: "twinkle",
      fxChars: "U",
      colors: { O: "#3f3428", F: "#c9b99f", L: "#f5ecdc", S: "#8e7d65", E: "#d99d3b", P: "#d49484", U: "#e8bd68", A: "#7e9e6d", R: "#d8ccb8", D: "#c2b39b" },
      rows: [
        "............................",
        "....UU......................",
        "...UUUU.....................",
        "....UU......................",
        "..............OOOOOOO.......",
        "...........OOOFSSSSSFOOO....",
        "...........OFSFFFFFFFSFO....",
        "..........OLFFFEFFFEFFFLO...",
        "..........OLLFFFPFFFFLLO....",
        "...........OLLLLLLLLLLO.....",
        "..........OFFFLLLLLLFFFO....",
        ".....A...OFSFFLLLLLLFFSFO...",
        "....AAA..OFFFLLLLLLLLFFFO...",
        ".....A..OOFSFFFFFFFFFFFSO...",
        ".......OFSFSFFLLLOOLLLFFO...",
        "........OOOOOOFFFOOFFFOO....",
        "..RRRRRRRRRRRRRRRRRRRRRRRR..",
        "..DDDDDDDDDDDDDDDDDDDDDDDD..",
      ],
    },
  };
  const svgCache = new Map();
  function renderCatSvg(theme, cls = "") {
    const art = catArt[theme];
    if (!art) return "";
    const key = theme + "|" + cls;
    if (svgCache.has(key)) return svgCache.get(key);
    const height = art.rows.length;
    const width = art.rows[0].length;
    const baseRects = [];
    const fxRects = [];
    for (let y = 0; y < height; y++) {
      const row = art.rows[y];
      let x = 0;
      while (x < width) {
        const ch = row[x];
        const color = art.colors[ch];
        if (!color) { x++; continue; }
        let run = 1;
        while (x + run < width && row[x + run] === ch) run++;
        const rect = `<rect x="${x}" y="${y}" width="${run}" height="1" fill="${color}"/>`;
        if (art.fxChars && art.fxChars.includes(ch)) fxRects.push(rect);
        else baseRects.push(rect);
        x += run;
      }
    }
    const fxGroup = fxRects.length ? `<g class="cat-fx cat-fx-${art.fx}">${fxRects.join("")}</g>` : "";
    const svg = `<svg class="pixel-cat-art${cls ? " " + cls : ""}" viewBox="0 0 ${width} ${height}" shape-rendering="crispEdges" aria-hidden="true">${baseRects.join("")}${fxGroup}</svg>`;
    svgCache.set(key, svg);
    return svg;
  }
  const accents = {
    blue: { dark: "#75abf5", light: "#245fb9" }, green: { dark: "#67c69a", light: "#187044" },
    amber: { dark: "#d6ad60", light: "#8a5b09" }, violet: { dark: "#b7a1e8", light: "#7044ab" },
    red: { dark: "#ed747e", light: "#ae2538" }, orange: { dark: "#e8a175", light: "#a34d16" },
    teal: { dark: "#6fc5c5", light: "#177276" }, rose: { dark: "#de96bb", light: "#a1326f" },
  };
  const colorKeys = ["canvas", "sidebar", "surface", "text", "border"];
  const root = document.documentElement;
  const mediaReduced = window.matchMedia?.("(prefers-reduced-motion: reduce)");
  const raf = window.requestAnimationFrame?.bind(window) || (callback => window.setTimeout(callback, 16));
  const cancelRaf = window.cancelAnimationFrame?.bind(window) || window.clearTimeout?.bind(window);
  let settings, persisted = true, dirty = false, dialog = null, opener = null, language = "", previewFrame = 0, previewPatch = null, saveTimer = 0, transitionTimer = 0, closeTimer = 0;
  let paletteSignature = "", currentPalette = null, activeTab = "appearance";
  let catDock = null, catBackdrop = null;
  const appliedStyles = new Map(), effectState = { snow: null, cat: null };
  let preferenceInputs = [], colorButtons = [], hexInputs = [], textNodes = [], labelNodes = [], tintButtons = [];
  let picker = null, pickerState = null, pickerPointer = null, pickerDrawFrame = 0, pickerPositionFrame = 0;
  const words = {
    ru: {
      title: "Настройки", subtitle: "Оформление, утилиты и поведение приложения", close: "Закрыть настройки",
      tabAppearance: "Оформление", tabWorkspace: "Диалоги", tabApp: "Приложение",
      theme: "Тема", classicThemes: "Базовые", catThemes: "С котиками", manulThemes: "С манулами",
      graphite: "Графит", charcoal: "Серая", light: "Светлая", crimson: "Красная",
      neko_night: "Ночной кот", neko_sakura: "Сакура-кот", neko_matcha: "Матча-кот", neko_cream: "Крем-кот",
      manul_steppe: "Степной манул", manul_snow: "Снежный манул", manul_altai: "Алтайский манул", manul_dune: "Песчаный манул",
      surfaceTint: "Тон интерфейса", surfaceTintNote: "Авто-адаптация всех поверхностей по одному цвету", tintNone: "По теме",
      ambientGradient: "Мягкий градиент", ambientGradientNote: "Легкое свечение поверхностей в тон темы",
      accent: "Акцент", blue: "Синий", green: "Зелёный", amber: "Янтарный", violet: "Сиреневый", red: "Красный", orange: "Оранжевый", teal: "Бирюзовый", rose: "Розовый",
      primaryStyle: "Главные кнопки", neutral: "Спокойные", accentButtons: "Акцентные",
      customPalette: "Тонкая настройка цветов", paletteNote: "Фон, панели, текст и границы вручную",
      canvas: "Фон окна", sidebar: "Сайдбар", surface: "Панели", text: "Текст", border: "Границы",
      resetColors: "Вернуть цвета темы", contrastNote: "Текст сливается с фоном", fixContrast: "Подобрать цвет текста", hexHint: "Цвет в формате #RRGGBB",
      corners: "Скругление", controlRadius: "Кнопки и поля", panelRadius: "Панели и сообщения",
      toolsGroup: "Инструменты в шапке", showTokenCounter: "Счетчик токенов", showTokenCounterNote: "Показывать примерный вес контекста в шапке чата",
      showSlimTool: "Очистка тяжелых шагов", showSlimToolNote: "Кнопка быстрого удаления thinking и больших выводов",
      reading: "Чтение и вид", density: "Плотность", comfortable: "Обычная", compact: "Компактная",
      textSize: "Размер текста", lineHeight: "Межстрочный интервал", width: "Ширина диалога", normal: "Обычная", wide: "Широкая", full: "Вся ширина",
      general: "Язык и анимации", language: "Язык", motion: "Анимации", motionSystem: "По системе", motionFull: "Плавные", motionReduced: "Без анимаций",
      effects: "Эффекты", snow: "Снег", snowNote: "Лёгкий эффект поверх окна", cat: "Кот на экране", catNote: "Пиксельный персонаж K1tty",
      effectsPaused: "Эффекты приостановлены, пока анимации отключены", reset: "Сбросить всё", done: "Готово",
      saved: "Сохраняется автоматически", temporary: "Действует до закрытия приложения",
      chooseColor: "Свой цвет", closeColor: "Закрыть палитру", hue: "Оттенок", saturation: "Насыщенность", brightness: "Яркость",
      pickerPlane: "Насыщенность и яркость", pickerHelp: "Влево и вправо: насыщенность, вверх и вниз: яркость. Shift меняет значение быстрее.",
      previousColor: "Вернуть предыдущий цвет", previous: "До", current: "Сейчас", quickColors: "Палитра",
      channelRed: "Красный канал", channelGreen: "Зелёный канал", channelBlue: "Синий канал",
    },
    uk: {
      title: "Налаштування", subtitle: "Оформлення, інструменти й поведінка застосунку", close: "Закрити налаштування",
      tabAppearance: "Оформлення", tabWorkspace: "Діалоги", tabApp: "Застосунок",
      theme: "Тема", classicThemes: "Базові", catThemes: "З котиками", manulThemes: "З манулами",
      graphite: "Графіт", charcoal: "Сіра", light: "Світла", crimson: "Червона",
      neko_night: "Нічний кіт", neko_sakura: "Сакура-кіт", neko_matcha: "Матча-кіт", neko_cream: "Крем-кіт",
      manul_steppe: "Степовий манул", manul_snow: "Сніговий манул", manul_altai: "Алтайський манул", manul_dune: "Піщаний манул",
      surfaceTint: "Тон інтерфейсу", surfaceTintNote: "Авто-адаптація всіх поверхонь за одним кольором", tintNone: "За темою",
      ambientGradient: "М'який градієнт", ambientGradientNote: "Легке сяйво поверхонь у тон теми",
      accent: "Акцент", blue: "Синій", green: "Зелений", amber: "Бурштиновий", violet: "Бузковий", red: "Червоний", orange: "Помаранчевий", teal: "Бірюзовий", rose: "Рожевий",
      primaryStyle: "Основні кнопки", neutral: "Стримані", accentButtons: "Акцентні",
      customPalette: "Тонке налаштування кольорів", paletteNote: "Тло, панелі, текст і межі вручну",
      canvas: "Тло вікна", sidebar: "Бічна панель", surface: "Панелі", text: "Текст", border: "Межі",
      resetColors: "Повернути кольори теми", contrastNote: "Текст зливається з тлом", fixContrast: "Підібрати колір тексту", hexHint: "Колір у форматі #RRGGBB",
      corners: "Заокруглення", controlRadius: "Кнопки й поля", panelRadius: "Панелі й повідомлення",
      toolsGroup: "Інструменти у шапці", showTokenCounter: "Лічильник токенів", showTokenCounterNote: "Показувати приблизну вагу контексту в шапці чату",
      showSlimTool: "Очищення важких кроків", showSlimToolNote: "Кнопка швидкого видалення thinking і великих виводів",
      reading: "Читання та вигляд", density: "Щільність", comfortable: "Звичайна", compact: "Компактна",
      textSize: "Розмір тексту", lineHeight: "Міжрядковий інтервал", width: "Ширина діалогу", normal: "Звичайна", wide: "Широка", full: "Уся ширина",
      general: "Мова та анімації", language: "Мова", motion: "Анімації", motionSystem: "За системою", motionFull: "Плавні", motionReduced: "Без анімацій",
      effects: "Ефекти", snow: "Сніг", snowNote: "Легкий ефект поверх вікна", cat: "Кіт на екрані", catNote: "Піксельний персонаж K1tty",
      effectsPaused: "Ефекти призупинено, поки анімації вимкнені", reset: "Скинути все", done: "Готово",
      saved: "Зберігається автоматично", temporary: "Діє до закриття застосунку",
      chooseColor: "Власний колір", closeColor: "Закрити палітру", hue: "Відтінок", saturation: "Насиченість", brightness: "Яскравість",
      pickerPlane: "Насиченість і яскравість", pickerHelp: "Ліворуч і праворуч: насиченість, угору й униз: яскравість. Shift змінює значення швидше.",
      previousColor: "Повернути попередній колір", previous: "До", current: "Зараз", quickColors: "Палітра",
      channelRed: "Червоний канал", channelGreen: "Зелений канал", channelBlue: "Синій канал",
    },
    en: {
      title: "Settings", subtitle: "Appearance, tools and app behavior", close: "Close settings",
      tabAppearance: "Appearance", tabWorkspace: "Conversations", tabApp: "Application",
      theme: "Theme", classicThemes: "Classic", catThemes: "Pixel cats", manulThemes: "Manuls",
      graphite: "Graphite", charcoal: "Charcoal", light: "Light", crimson: "Crimson",
      neko_night: "Midnight Cat", neko_sakura: "Sakura Cat", neko_matcha: "Matcha Cat", neko_cream: "Cream Cat",
      manul_steppe: "Steppe Manul", manul_snow: "Snow Manul", manul_altai: "Altai Manul", manul_dune: "Dune Manul",
      surfaceTint: "Interface tint", surfaceTintNote: "Auto-adapt all surfaces from a single color", tintNone: "Theme default",
      ambientGradient: "Soft gradient", ambientGradientNote: "Subtle atmospheric surface glow matching the theme",
      accent: "Accent", blue: "Blue", green: "Green", amber: "Amber", violet: "Violet", red: "Red", orange: "Orange", teal: "Teal", rose: "Rose",
      primaryStyle: "Primary buttons", neutral: "Neutral", accentButtons: "Accent",
      customPalette: "Manual color overrides", paletteNote: "Backgrounds, panels, text and borders",
      canvas: "Window background", sidebar: "Sidebar", surface: "Panels", text: "Text", border: "Borders",
      resetColors: "Restore theme colors", contrastNote: "The text blends into the background", fixContrast: "Choose readable text", hexHint: "Color in #RRGGBB format",
      corners: "Corner radius", controlRadius: "Buttons and fields", panelRadius: "Panels and messages",
      toolsGroup: "Header tools", showTokenCounter: "Token counter", showTokenCounterNote: "Show estimated context size in the chat header",
      showSlimTool: "Heavy-step cleaner", showSlimToolNote: "Quick button to strip thinking blocks and oversized outputs",
      reading: "Reading layout", density: "Density", comfortable: "Comfortable", compact: "Compact",
      textSize: "Text size", lineHeight: "Line spacing", width: "Reading width", normal: "Standard", wide: "Wide", full: "Full width",
      general: "Language and motion", language: "Language", motion: "Motion", motionSystem: "Follow OS", motionFull: "Smooth", motionReduced: "None",
      effects: "Effects", snow: "Snow", snowNote: "A light effect over the window", cat: "On-screen cat", catNote: "The K1tty pixel character",
      effectsPaused: "Effects pause while motion is disabled", reset: "Reset all", done: "Done",
      saved: "Changes save automatically", temporary: "Preferences last until the app closes",
      chooseColor: "Custom color", closeColor: "Close color picker", hue: "Hue", saturation: "Saturation", brightness: "Brightness",
      pickerPlane: "Saturation and brightness", pickerHelp: "Left and right change saturation; up and down change brightness. Hold Shift for larger steps.",
      previousColor: "Restore previous color", previous: "Before", current: "Current", quickColors: "Palette",
      channelRed: "Red channel", channelGreen: "Green channel", channelBlue: "Blue channel",
    },
  };
  function defaultAccentFor(theme) { return themeAccents[theme] || "blue"; }
  function hex(value) {
    if (typeof value !== "string") return null;
    const raw = value.trim().toLowerCase();
    const v = raw.startsWith("#") ? raw : "#" + raw;
    if (/^#[a-f0-9]{6}$/.test(v)) return v;
    if (/^#[a-f0-9]{3}$/.test(v)) return "#" + [...v.slice(1)].map(c => c + c).join("");
    return null;
  }
  function normalize(value) {
    const v = value && typeof value === "object" && !Array.isArray(value) ? value : {};
    const customColors = {};
    for (const key of colorKeys) { const c = hex(v.customColors?.[key]); if (c) customColors[key] = c; }
    const numeric = (key, min, max, step = 1) => typeof v[key] === "number" && Number.isFinite(v[key]) && v[key] >= min && v[key] <= max ? Math.round(v[key] / step) * step : defaults[key];
    return {
      theme: Object.hasOwn(palettes, v.theme) ? v.theme : defaults.theme,
      accent: v.accent === "custom" || Object.hasOwn(accents, v.accent) ? v.accent : defaults.accent,
      surfaceTint: hex(v.surfaceTint) || "",
      ambientGradient: v.ambientGradient !== false,
      primaryStyle: ["neutral", "accent"].includes(v.primaryStyle) ? v.primaryStyle : defaults.primaryStyle,
      customAccent: hex(v.customAccent) || defaults.customAccent,
      customColors,
      density: ["comfortable", "compact"].includes(v.density) ? v.density : defaults.density,
      textSize: numeric("textSize", 12, 20),
      lineHeight: Number(numeric("lineHeight", 1.4, 2, .1).toFixed(1)),
      readingWidth: ["normal", "wide", "full"].includes(v.readingWidth) ? v.readingWidth : defaults.readingWidth,
      controlRadius: numeric("controlRadius", 0, 16),
      panelRadius: numeric("panelRadius", 0, 20),
      motion: ["system", "full", "reduced"].includes(v.motion) ? v.motion : defaults.motion,
      snow: v.snow === true,
      cat: v.cat === true,
      showTokenCounter: v.showTokenCounter !== false,
      showSlimTool: v.showSlimTool !== false,
    };
  }
  settings = normalize(null);
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) || "null");
    settings = normalize(saved);
    if (saved?.theme === "system") localStorage.setItem(KEY, JSON.stringify(settings));
  } catch {}
  function copySettings() { return { ...settings, customColors: { ...settings.customColors } }; }
  function rgb(color) { return [1, 3, 5].map(start => parseInt(color.slice(start, start + 2), 16)); }
  function fromRgb(values) { return "#" + values.map(v => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0")).join(""); }
  function toHsl(color) {
    const [r, g, b] = rgb(color).map(v => v / 255);
    const max = Math.max(r, g, b), min = Math.min(r, g, b), d = max - min, l = (max + min) / 2;
    let h = 0, s = 0;
    if (d) {
      s = l > .5 ? d / (2 - max - min) : d / (max + min);
      h = max === r ? ((g - b) / d + (g < b ? 6 : 0)) : max === g ? ((b - r) / d + 2) : ((r - g) / d + 4);
      h *= 60;
    }
    return { h, s, l };
  }
  function fromHsl(h, s, l) {
    const hue = ((h % 360) + 360) % 360;
    const sat = Math.max(0, Math.min(1, s));
    const light = Math.max(0, Math.min(1, l));
    const c = (1 - Math.abs(2 * light - 1)) * sat;
    const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
    const m = light - c / 2;
    const ch = hue < 60 ? [c, x, 0] : hue < 120 ? [x, c, 0] : hue < 180 ? [0, c, x] : hue < 240 ? [0, x, c] : hue < 300 ? [x, 0, c] : [c, 0, x];
    return fromRgb(ch.map(v => (v + m) * 255));
  }
  function mix(a, b, amount) { const left = rgb(a), right = rgb(b); return fromRgb(left.map((v, i) => v + (right[i] - v) * amount)); }
  function luminance(color) { return rgb(color).map(v => v / 255).map(v => v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4).reduce((sum, value, i) => sum + value * [.2126, .7152, .0722][i], 0); }
  function contrast(a, b) { const x = luminance(a), y = luminance(b); return (Math.max(x, y) + .05) / (Math.min(x, y) + .05); }
  function inkFor(color) { return contrast("#111112", color) >= contrast("#ffffff", color) ? "#111112" : "#ffffff"; }
  function readable(color, backgrounds, minimum = 4.5) {
    const score = candidate => Math.min(...backgrounds.map(bg => contrast(candidate, bg)));
    if (score(color) >= minimum) return color;
    const end = score("#111112") > score("#ffffff") ? "#111112" : "#ffffff";
    for (let step = 1; step <= 20; step++) { const result = mix(color, end, step / 20); if (score(result) >= minimum) return result; }
    return end;
  }
  function deriveTintedPalette(base, tintColor) {
    const isLight = luminance(base.canvas) > .45;
    const { h, s } = toHsl(tintColor);
    const sat = Math.max(.08, Math.min(s, isLight ? .34 : .46));
    if (isLight) {
      const sidebar = fromHsl(h, sat * .62, .925);
      const canvas = fromHsl(h, sat * .58, .958);
      const surface = fromHsl(h, sat * .44, .992);
      const inset = fromHsl(h, sat * .60, .936);
      const hover = fromHsl(h, sat * .56, .898);
      const raised = fromHsl(h, sat * .54, .875);
      const user = fromHsl(h, sat * .54, .932);
      const border = fromHsl(h, sat * .44, .842);
      const strong = fromHsl(h, sat * .44, .752);
      const text = fromHsl(h, Math.min(sat * .45, .22), .135);
      const bgs = [canvas, sidebar, surface, raised];
      return {
        sidebar, canvas, surface, hover, raised, inset, user, border, strong, text,
        secondary: readable(fromHsl(h, Math.min(sat * .35, .20), .34), bgs),
        muted: readable(fromHsl(h, Math.min(sat * .30, .18), .43), bgs),
      };
    }
    const sidebar = fromHsl(h, sat * .70, .068);
    const canvas = fromHsl(h, sat * .76, .088);
    const surface = fromHsl(h, sat * .72, .118);
    const inset = fromHsl(h, sat * .70, .074);
    const hover = fromHsl(h, sat * .66, .158);
    const raised = fromHsl(h, sat * .64, .184);
    const user = fromHsl(h, sat * .70, .138);
    const border = fromHsl(h, sat * .56, .218);
    const strong = fromHsl(h, sat * .52, .308);
    const text = fromHsl(h, Math.min(sat * .32, .20), .94);
    const bgs = [canvas, sidebar, surface, raised];
    return {
      sidebar, canvas, surface, hover, raised, inset, user, border, strong, text,
      secondary: readable(fromHsl(h, Math.min(sat * .30, .20), .74), bgs),
      muted: readable(fromHsl(h, Math.min(sat * .26, .18), .60), bgs),
    };
  }
  function palette() {
    const signature = JSON.stringify([settings.theme, settings.accent, settings.surfaceTint, settings.ambientGradient, settings.primaryStyle, settings.customAccent, settings.customColors]);
    if (signature === paletteSignature) return currentPalette;
    paletteSignature = signature;
    const base = palettes[settings.theme];
    const tinted = settings.surfaceTint ? deriveTintedPalette(base, settings.surfaceTint) : base;
    const p = { ...tinted, ...settings.customColors };
    const custom = Object.keys(settings.customColors).length > 0;
    p.light = luminance(p.canvas) > .45;
    if (custom) {
      p.hover = mix(p.surface, p.text, .06); p.raised = mix(p.surface, p.text, .10);
      p.inset = mix(p.canvas, p.sidebar, .5); p.user = mix(p.surface, p.text, .025);
      p.strong = mix(p.border, p.text, .18);
      const backgrounds = [p.canvas, p.sidebar, p.surface, p.raised];
      p.secondary = readable(mix(p.text, p.surface, .24), backgrounds);
      p.muted = readable(mix(p.text, p.surface, .42), backgrounds);
    }
    p.accent = settings.accent === "custom" ? settings.customAccent : accents[settings.accent][p.light ? "light" : "dark"];
    p.primaryBackground = settings.primaryStyle === "accent" ? p.accent : p.text;
    p.primaryInk = inkFor(p.primaryBackground); p.accentInk = inkFor(p.accent);
    p.primaryHover = mix(p.primaryBackground, p.primaryInk === "#ffffff" ? "#000000" : "#ffffff", .08);
    p.primaryHoverInk = inkFor(p.primaryHover);
    p.contrastLow = [p.canvas, p.sidebar, p.surface].some(bg => contrast(p.text, bg) < 4.5);
    const glowBase = settings.surfaceTint || p.accent;
    const glowHsl = toHsl(glowBase);
    p.glowColor = fromHsl(glowHsl.h, Math.min(.75, Math.max(.35, glowHsl.s)), p.light ? .55 : .52);
    p.glowSecondary = fromHsl(glowHsl.h + 28, Math.min(.68, Math.max(.30, glowHsl.s)), p.light ? .58 : .48);
    currentPalette = p;
    return p;
  }
  function isReducedMotion() { return settings.motion === "reduced" || (settings.motion === "system" && !!mediaReduced?.matches); }
  function t(key) { return (words[language] || words.en)[key] || key; }
  function emit() { window.dispatchEvent(new CustomEvent("k1tty:appearancechange", { detail: { ...copySettings(), resolvedTheme: settings.theme, reducedMotion: isReducedMotion() } })); }
  function syncEffects() {
    const reduced = isReducedMotion();
    for (const [key, effect] of [["snow", window.K1ttyWinter], ["cat", window.K1ttyCat]]) {
      const enabled = settings[key] && !reduced;
      if (effect && effect.isEnabled() !== enabled) effect.setEnabled(enabled);
      if (effectState[key] === enabled) continue;
      effectState[key] = enabled;
      try { localStorage.setItem(key === "snow" ? "k1tty_winter" : "k1tty_cat", enabled ? "1" : "0"); } catch {}
    }
  }
  function syncCatDecor() {
    const art = catArt[settings.theme];
    const hasCat = !!art;
    setData("catTheme", hasCat ? settings.theme : "none");
    if (!document.body) return;
    if (!hasCat) {
      if (catDock) catDock.hidden = true;
      if (catBackdrop) catBackdrop.hidden = true;
      return;
    }
    const drawerInner = document.querySelector(".drawer-inner");
    const drawerFoot = document.querySelector(".drawer-foot");
    if (drawerInner && drawerFoot) {
      if (!catDock) {
        catDock = document.createElement("div");
        catDock.id = "themeCatDock";
        catDock.className = "theme-cat-dock";
        catDock.setAttribute("aria-hidden", "true");
        drawerInner.insertBefore(catDock, drawerFoot);
      }
      if (catDock.dataset.theme !== settings.theme) {
        catDock.dataset.theme = settings.theme;
        catDock.innerHTML = `<span class="theme-cat-dock-tag">${art.tag}</span><span class="theme-cat-dock-art">${renderCatSvg(settings.theme, "cat-art-dock")}</span>`;
      }
      catDock.hidden = false;
    }
    const mainBody = document.querySelector(".main-body");
    if (mainBody) {
      if (!catBackdrop) {
        catBackdrop = document.createElement("div");
        catBackdrop.id = "themeCatBackdrop";
        catBackdrop.className = "theme-cat-backdrop";
        catBackdrop.setAttribute("aria-hidden", "true");
        mainBody.appendChild(catBackdrop);
      }
      if (catBackdrop.dataset.theme !== settings.theme) {
        catBackdrop.dataset.theme = settings.theme;
        catBackdrop.innerHTML = renderCatSvg(settings.theme, "cat-art-backdrop");
      }
      catBackdrop.hidden = false;
    }
  }
  function setData(key, value) { if (root.dataset[key] !== value) root.dataset[key] = value; }
  function apply({ animate = false } = {}) {
    const oldPalette = currentPalette, p = palette(), reduced = isReducedMotion();
    if (animate && oldPalette && oldPalette !== p && !reduced && document.visibilityState !== "hidden") {
      setData("themeTransition", "on");
      window.clearTimeout(transitionTimer);
      transitionTimer = window.setTimeout(() => { delete root.dataset.themeTransition; transitionTimer = 0; }, 220);
    } else if (reduced || (oldPalette && oldPalette !== p && !animate)) { delete root.dataset.themeTransition; window.clearTimeout(transitionTimer); transitionTimer = 0; }
    setData("appearance", settings.theme); setData("theme", settings.theme); setData("colorScheme", p.light ? "light" : "dark");
    setData("density", settings.density); setData("motion", reduced ? "reduced" : "full"); setData("readingWidth", settings.readingWidth);
    setData("ambientGradient", settings.ambientGradient ? "on" : "off");
    setData("surfaceTint", settings.surfaceTint ? "on" : "off");
    if (root.style.colorScheme !== (p.light ? "light" : "dark")) root.style.colorScheme = p.light ? "light" : "dark";
    const accentRgb = rgb(p.accent).join(","), textRgb = rgb(p.text).join(",");
    const glowRgb = rgb(p.glowColor).join(","), glow2Rgb = rgb(p.glowSecondary).join(",");
    const tintStrength = settings.surfaceTint ? (p.light ? .16 : .18) : (p.light ? .08 : .11);
    const tintStrength2 = settings.surfaceTint ? (p.light ? .10 : .12) : (p.light ? .05 : .07);
    for (const [key, value] of Object.entries({
      "bg-sidebar": p.sidebar, "bg-canvas": p.canvas, "bg-surface": p.surface, "bg-hover": p.hover, "bg-raised": p.raised, "bg-inset": p.inset,
      "txt-1": p.text, "txt-2": p.secondary, "txt-3": p.muted, border: p.border, "border-strong": p.strong,
      "on-primary": p.primaryInk, "primary-bg": p.primaryBackground, "primary-hover": p.primaryHover, "primary-hover-ink": p.primaryHoverInk, "accent-ink": p.accentInk, "bg-user": p.user, "modal-backdrop": p.light ? "rgba(30,30,35,.26)" : "rgba(0,0,0,.52)", "scroll-thumb": `rgba(${textRgb},.26)`,
      accent: p.accent, "accent-soft": `rgba(${accentRgb},${p.light ? ".10" : ".14"})`, "accent-selection": `rgba(${accentRgb},.22)`,
      "ambient-glow": `rgba(${glowRgb},${tintStrength})`, "ambient-glow-2": `rgba(${glow2Rgb},${tintStrength2})`, "ambient-glow-soft": `rgba(${glowRgb},${(tintStrength * .78).toFixed(3)})`,
      "chat-font-size": settings.textSize + "px", "code-font-size": Math.max(11, settings.textSize - 1) + "px", "chat-line-height": String(settings.lineHeight),
      "reading-width": settings.readingWidth === "full" ? "100%" : settings.readingWidth === "wide" ? "1040px" : "800px",
      "control-radius": settings.controlRadius + "px", "panel-radius": settings.panelRadius + "px", "inner-radius": Math.max(0, settings.panelRadius - 4) + "px",
      ok: p.light ? "#187044" : "#67c69a", warn: p.light ? "#8a5b09" : "#d6ad60", err: p.light ? "#b92c37" : "#ef7d87",
      "ok-soft": p.light ? "rgba(24,112,68,.08)" : "rgba(103,198,154,.11)", "warn-soft": p.light ? "rgba(138,91,9,.08)" : "rgba(214,173,96,.11)", "err-soft": p.light ? "rgba(185,44,55,.08)" : "rgba(239,125,135,.11)",
    })) {
      if (appliedStyles.get(key) === value) continue;
      appliedStyles.set(key, value); root.style.setProperty("--" + key, value);
    }
    syncCatDecor(); syncEffects(); syncControls();
  }
  function persist() {
    window.clearTimeout(saveTimer); saveTimer = 0;
    if (!dirty) return;
    try { localStorage.setItem(KEY, JSON.stringify(settings)); persisted = true; dirty = false; } catch { persisted = false; }
    syncSaveNote();
  }
  function update(patch, { preview = false, animate = true } = {}) {
    if (!patch || typeof patch !== "object") return copySettings();
    let next = { ...settings, ...patch };
    if (patch.theme && patch.theme !== settings.theme) {
      if (!Object.hasOwn(patch, "customColors")) next.customColors = {};
      if (!Object.hasOwn(patch, "surfaceTint")) next.surfaceTint = "";
      if (!Object.hasOwn(patch, "accent")) next.accent = defaultAccentFor(patch.theme);
      if (!Object.hasOwn(patch, "primaryStyle")) next.primaryStyle = patch.theme === "crimson" ? "accent" : "neutral";
    }
    next = normalize(next);
    if (JSON.stringify(next) === JSON.stringify(settings)) { if (!preview) persist(); return copySettings(); }
    settings = next; dirty = true;
    apply({ animate: animate && !preview }); emit();
    if (preview) { window.clearTimeout(saveTimer); saveTimer = window.setTimeout(persist, 180); }
    else persist();
    return copySettings();
  }
  function queuePreview(patch) {
    const previous = previewPatch;
    previewPatch = { ...previous, ...patch };
    if (patch.customColors) previewPatch.customColors = { ...settings.customColors, ...previous?.customColors, ...patch.customColors };
    if (!previewFrame) previewFrame = raf(flushPreview);
  }
  function flushPreview() {
    if (previewFrame) cancelRaf?.(previewFrame);
    previewFrame = 0;
    if (!previewPatch) return;
    const patch = previewPatch; previewPatch = null;
    update(patch, { preview: true });
  }
  function settlePreview() { flushPreview(); persist(); }
  function labelButton() {
    const button = document.getElementById("appearanceBtn");
    if (button) { button.title = t("title"); button.setAttribute("aria-label", t("title")); button.setAttribute("aria-haspopup", "dialog"); }
  }
  function syncSegGroup(group, immediate = false) {
    if (!group) return;
    const thumb = group.querySelector(".appearance-seg-thumb");
    if (!thumb) return;
    const active = group.querySelector("input:checked")?.closest(".appearance-choice")
      || group.querySelector('[aria-checked="true"]')
      || group.querySelector('[aria-selected="true"]');
    if (!active || active.offsetWidth === 0) return;
    if (immediate) thumb.classList.add("no-anim");
    thumb.style.width = active.offsetWidth + "px";
    thumb.style.transform = `translate3d(${active.offsetLeft}px, 0, 0)`;
    thumb.style.opacity = "1";
    if (immediate) {
      void thumb.offsetWidth;
      thumb.classList.remove("no-anim");
    }
  }
  function syncAllSegs(immediate = false) {
    if (!dialog || !dialog.open) return;
    dialog.querySelectorAll("[data-seg-group]").forEach(group => syncSegGroup(group, immediate));
  }
  function setTab(name, immediate = false) {
    if (!dialog) return;
    activeTab = name;
    dialog.querySelectorAll("[data-appearance-tab]").forEach(btn => {
      const on = btn.dataset.appearanceTab === name;
      btn.setAttribute("aria-selected", String(on));
      btn.tabIndex = on ? 0 : -1;
    });
    dialog.querySelectorAll("[data-appearance-panel]").forEach(panel => {
      const on = panel.dataset.appearancePanel === name;
      panel.hidden = !on;
      panel.classList.toggle("is-active", on);
    });
    closePicker(false);
    raf(() => syncAllSegs(immediate));
  }
  function setLanguage(lang) {
    const next = Object.hasOwn(words, lang) ? lang : "en";
    const changed = language !== next;
    language = next; labelButton();
    if (!dialog) return;
    if (changed || dialog.lang !== language) {
      dialog.lang = language;
      textNodes.forEach(el => { el.textContent = t(el.dataset.appearanceText); });
      labelNodes.forEach(el => { el.setAttribute("aria-label", t(el.dataset.appearanceLabel)); el.title = t(el.dataset.appearanceLabel); });
      dialog.querySelectorAll("[data-appearance-language]").forEach(button => {
        const active = button.dataset.appearanceLanguage === language;
        button.setAttribute("aria-checked", String(active));
        button.tabIndex = active ? 0 : -1;
      });
    }
    syncControls();
    raf(() => syncAllSegs(false));
  }
  function setText(element, value) { if (element && element.textContent !== value) element.textContent = value; }
  function syncSaveNote() { if (dialog) setText(dialog.querySelector("#appearanceSaveNote"), t(persisted ? "saved" : "temporary")); }
  function syncControls() {
    if (!dialog) return;
    const p = palette();
    for (const input of preferenceInputs) {
      const value = settings[input.dataset.pref];
      if (input.type === "radio") { if (input.checked !== (value === input.value)) input.checked = value === input.value; }
      else if (input.type === "checkbox") { if (input.checked !== !!value) input.checked = !!value; }
      else if (input.value !== String(value)) input.value = String(value);
    }
    for (const btn of tintButtons) {
      const target = btn.dataset.tintPreset;
      const on = target === settings.surfaceTint;
      btn.setAttribute("aria-checked", String(on));
    }
    const customTintBtn = dialog.querySelector('[data-color-open="surfaceTint"]');
    if (customTintBtn) {
      const isCustomTint = !!settings.surfaceTint && !tintPresets.some(x => x.key === settings.surfaceTint);
      customTintBtn.classList.toggle("is-active", isCustomTint);
      customTintBtn.style.setProperty("--tint-custom", settings.surfaceTint || p.accent);
    }
    const colors = { ...p, accent: p.accent, surfaceTint: settings.surfaceTint || p.accent };
    for (const button of colorButtons) {
      const value = colors[button.dataset.colorOpen];
      if (value && button.dataset.colorValue !== value) { button.dataset.colorValue = value; button.style.setProperty("--color-swatch", value); }
    }
    for (const input of hexInputs) { if (document.activeElement !== input && input.value !== colors[input.dataset.colorKey]) input.value = colors[input.dataset.colorKey]; }
    for (const [id, value] of [["Text", settings.textSize + " px"], ["ControlRadius", settings.controlRadius + " px"], ["PanelRadius", settings.panelRadius + " px"], ["LineHeight", settings.lineHeight.toFixed(1)]]) {
      setText(dialog.querySelector("#appearance" + id + "Value"), value);
      dialog.querySelector("#appearance" + (id === "Text" ? "TextSize" : id))?.setAttribute("aria-valuetext", value);
    }
    dialog.querySelector("#appearanceEffectsNote").hidden = !isReducedMotion();
    dialog.querySelector("#appearanceContrastNote").hidden = !p.contrastLow;
    if (pickerState) {
      const selected = colors[pickerState.key];
      if (!previewPatch && selected && selected !== pickerState.value) { pickerState.value = selected; pickerState.hsv = toHsv(selected, pickerState.hsv.h); }
      drawPicker();
    }
    syncSaveNote();
    if (dialog.open) raf(() => syncAllSegs(false));
  }
  function radio(name, value, text) { return `<label class="appearance-choice"><input type="radio" name="appearance-${name}" data-pref="${name}" value="${value}"><span data-appearance-text="${text || value}"></span></label>`; }
  function seg(content, extra = "") { return `<div class="appearance-options${extra ? " " + extra : ""}" data-seg-group><span class="appearance-seg-thumb" aria-hidden="true"></span>${content}</div>`; }
  function themeTile(name) {
    const hasCat = Object.hasOwn(catArt, name);
    const catMarkup = hasCat ? `<span class="appearance-theme-cat">${renderCatSvg(name, "cat-art-tile")}</span>` : "";
    return `<label class="appearance-theme${hasCat ? " is-cat-theme" : ""}"><input type="radio" name="appearance-theme" data-pref="theme" value="${name}"><span class="appearance-theme-preview appearance-theme-${name}" aria-hidden="true"><span class="appearance-mini-sidebar"><i></i><i></i><i></i></span><span class="appearance-mini-content"><i></i><i></i></span>${catMarkup}</span><span class="appearance-theme-name" data-appearance-text="${name}"></span></label>`;
  }
  function effect(name, note) { return `<label class="appearance-effect"><span class="appearance-effect-copy"><span data-appearance-text="${name}"></span><small data-appearance-text="${note}"></small></span><input type="checkbox" role="switch" data-pref="${name}"><span class="appearance-switch" aria-hidden="true"></span></label>`; }
  function range(key, id, min, max, step = 1) { return `<div class="appearance-row appearance-size-row"><label for="appearance${id}" data-appearance-text="${key}"></label><div class="appearance-size-controls"><input id="appearance${id}" type="range" min="${min}" max="${max}" step="${step}" data-pref="${key}"><output id="appearance${id === "TextSize" ? "Text" : id}Value" for="appearance${id}"></output></div></div>`; }
  function colorField(key) { return `<div class="appearance-color-field"><label for="appearanceHex-${key}" data-appearance-text="${key}"></label><div class="appearance-color-control"><button type="button" id="appearanceColor-${key}" class="appearance-color-swatch" data-color-open="${key}" data-appearance-label="${key}" aria-haspopup="dialog" aria-expanded="false" aria-controls="appearanceColorPicker"><span aria-hidden="true"></span></button><input type="text" id="appearanceHex-${key}" class="appearance-hex" data-color-key="${key}" maxlength="7" pattern="#?[0-9a-fA-F]{3}([0-9a-fA-F]{3})?" spellcheck="false" autocomplete="off" autocapitalize="off" data-appearance-label="hexHint" aria-labelledby="appearanceHexLabel-${key}"><span id="appearanceHexLabel-${key}" class="appearance-sr" data-appearance-text="${key}"></span></div></div>`; }
  function toHsv(color, lastHue = 0) {
    const [r, g, b] = rgb(color).map(value => value / 255), max = Math.max(r, g, b), min = Math.min(r, g, b), delta = max - min;
    let hue = lastHue;
    if (delta) {
      hue = max === r ? ((g - b) / delta) % 6 : max === g ? (b - r) / delta + 2 : (r - g) / delta + 4;
      hue = ((hue * 60) + 360) % 360;
    }
    return { h: hue, s: max ? delta / max : 0, v: max };
  }
  function fromHsv({ h, s, v }) {
    const hue = ((h % 360) + 360) % 360 / 60, chroma = v * s, x = chroma * (1 - Math.abs(hue % 2 - 1)), offset = v - chroma;
    const channels = hue < 1 ? [chroma, x, 0] : hue < 2 ? [x, chroma, 0] : hue < 3 ? [0, chroma, x] : hue < 4 ? [0, x, chroma] : hue < 5 ? [x, 0, chroma] : [chroma, 0, x];
    return fromRgb(channels.map(value => Math.round((value + offset) * 255 + 1e-9)));
  }
  function colorPatch(key, value) {
    if (key === "accent") return { accent: "custom", customAccent: value };
    if (key === "surfaceTint") return { surfaceTint: value, customColors: {} };
    return { customColors: { [key]: value } };
  }
  function drawPicker() {
    if (pickerDrawFrame) cancelRaf?.(pickerDrawFrame);
    pickerDrawFrame = 0;
    if (!pickerState || !picker) return;
    const { hsv, value, previous, key } = pickerState, p = palette();
    picker.style.setProperty("--picker-hue", `hsl(${hsv.h} 100% 50%)`);
    picker.style.setProperty("--picker-saturation", `${hsv.s * 100}%`);
    picker.style.setProperty("--picker-value", `${(1 - hsv.v) * 100}%`);
    picker.style.setProperty("--picker-current", value);
    picker.style.setProperty("--picker-previous", previous);
    picker.style.setProperty("--picker-text", readable(p.text, [p.surface]));
    setText(picker.querySelector("#appearancePickerTitle"), t(key));
    const plane = picker.querySelector("#appearancePickerPlane");
    plane.setAttribute("aria-valuenow", String(Math.round(hsv.s * 100)));
    plane.setAttribute("aria-valuetext", `${t("saturation")} ${Math.round(hsv.s * 100)}%, ${t("brightness")} ${Math.round(hsv.v * 100)}%`);
    const hue = picker.querySelector("#appearancePickerHue");
    if (document.activeElement !== hue && hue.value !== String(hsv.h)) hue.value = String(hsv.h);
    hue.setAttribute("aria-valuetext", Math.round(hsv.h) + "°");
    const hexInput = picker.querySelector("#appearancePickerHex");
    if (document.activeElement !== hexInput && hexInput.value !== value) hexInput.value = value;
    const channels = rgb(value);
    picker.querySelectorAll("[data-picker-channel]").forEach((input, index) => { if (document.activeElement !== input && input.value !== String(channels[index])) input.value = String(channels[index]); });
    picker.querySelectorAll("[data-picker-swatch]").forEach(button => button.setAttribute("aria-pressed", String(button.dataset.pickerSwatch === value)));
    picker.querySelector("[data-picker-previous]").title = t("previousColor") + " · " + previous.toUpperCase();
  }
  function previewPicker(value, retainHsv = false) {
    if (!pickerState) return;
    const color = hex(value);
    if (!color) return;
    if (!retainHsv) pickerState.hsv = toHsv(color, pickerState.hsv.h);
    pickerState.value = color;
    queuePreview(colorPatch(pickerState.key, color));
    if (!pickerDrawFrame) pickerDrawFrame = raf(() => { pickerDrawFrame = 0; drawPicker(); });
  }
  function positionPicker() {
    pickerPositionFrame = 0;
    if (!pickerState || !dialog.open) return;
    const bounds = dialog.getBoundingClientRect(), anchor = pickerState.opener.getBoundingClientRect();
    const width = dialog.clientWidth || bounds.width || Math.min(648, window.innerWidth - 48);
    const height = dialog.clientHeight || bounds.height || Math.max(1, window.innerHeight - 64);
    const inset = 12;
    picker.style.width = Math.min(304, Math.max(1, width - inset * 2)) + "px";
    picker.style.maxHeight = Math.max(1, height - inset * 2) + "px";
    const size = picker.getBoundingClientRect(), panelWidth = size.width || Math.min(304, width - inset * 2), panelHeight = size.height || Math.min(416, height - inset * 2);
    const left = Math.max(inset, Math.min(anchor.left - bounds.left, width - panelWidth - inset));
    let top = anchor.bottom - bounds.top + 8;
    if (top + panelHeight > height - inset) top = anchor.top - bounds.top - panelHeight - 8;
    top = Math.max(inset, Math.min(top, height - panelHeight - inset));
    picker.style.left = left + "px"; picker.style.top = top + "px";
  }
  function stopPickerPointer() {
    if (!pickerPointer) return;
    const pointer = pickerPointer; pickerPointer = null;
    try { pointer.element.releasePointerCapture?.(pointer.id); } catch {}
  }
  function closePicker(restoreFocus = true) {
    if (!pickerState) return;
    const previousOpener = pickerState.opener;
    stopPickerPointer(); settlePreview();
    pickerState = null;
    if (pickerDrawFrame) cancelRaf?.(pickerDrawFrame);
    if (pickerPositionFrame) cancelRaf?.(pickerPositionFrame);
    pickerDrawFrame = pickerPositionFrame = 0;
    picker.hidden = true;
    previousOpener.setAttribute("aria-expanded", "false");
    if (restoreFocus && dialog.open && previousOpener.isConnected) previousOpener.focus({ preventScroll: true });
  }
  function openPicker(button) {
    if (pickerState?.opener === button) { closePicker(); return; }
    closePicker(false); settlePreview();
    const key = button.dataset.colorOpen;
    if (![...colorKeys, "accent", "surfaceTint"].includes(key)) return;
    const p = palette();
    const value = key === "surfaceTint" ? (settings.surfaceTint || p.accent) : p[key];
    pickerState = { key, value, previous: value, hsv: toHsv(value), opener: button };
    button.setAttribute("aria-expanded", "true");
    picker.hidden = false;
    picker.querySelectorAll("[aria-invalid]").forEach(input => input.removeAttribute("aria-invalid"));
    drawPicker(); positionPicker();
    pickerPositionFrame = raf(positionPicker);
    picker.querySelector("#appearancePickerPlane").focus({ preventScroll: true });
  }
  function pickerMarkup() {
    const swatches = ["#ffffff", "#141415", "#ed747e", "#d6ad60", "#67c69a", "#6fc5c5", "#75abf5", "#b7a1e8"];
    return `<section class="appearance-color-picker" id="appearanceColorPicker" role="dialog" aria-labelledby="appearancePickerTitle" hidden>
      <div class="appearance-picker-head"><strong id="appearancePickerTitle"></strong><button type="button" class="appearance-picker-close" data-picker-close data-appearance-label="closeColor"><svg class="ico" aria-hidden="true"><use href="#i-x"/></svg></button></div>
      <div id="appearancePickerPlane" class="appearance-picker-plane" role="slider" tabindex="0" aria-valuemin="0" aria-valuemax="100" data-appearance-label="pickerPlane" aria-describedby="appearancePickerHelp"><span class="appearance-picker-cursor" aria-hidden="true"></span></div>
      <span id="appearancePickerHelp" class="appearance-sr" data-appearance-text="pickerHelp"></span>
      <input id="appearancePickerHue" class="appearance-picker-hue" type="range" min="0" max="360" step="1" data-appearance-label="hue">
      <div class="appearance-picker-values"><label class="appearance-picker-hex-label">HEX<input type="text" id="appearancePickerHex" maxlength="7" spellcheck="false" autocomplete="off" autocapitalize="off" data-appearance-label="hexHint"></label>${[["r", "R", "channelRed"], ["g", "G", "channelGreen"], ["b", "B", "channelBlue"]].map(([key, label, name]) => `<label>${label}<input type="number" min="0" max="255" step="1" inputmode="numeric" data-picker-channel="${key}" data-appearance-label="${name}"></label>`).join("")}</div>
      <div class="appearance-picker-swatches" role="group" data-appearance-label="quickColors">${swatches.map(color => `<button type="button" data-picker-swatch="${color}" aria-label="${color.toUpperCase()}" aria-pressed="false" style="--picker-swatch:${color}"></button>`).join("")}</div>
      <div class="appearance-picker-foot"><div class="appearance-picker-comparison"><button type="button" data-picker-previous data-appearance-label="previousColor"><span class="appearance-picker-before" aria-hidden="true"></span><small data-appearance-text="previous"></small></button><span><span class="appearance-picker-current" aria-hidden="true"></span><small data-appearance-text="current"></small></span></div><button type="button" class="btn" data-picker-close data-appearance-text="done"></button></div>
    </section>`;
  }
  function mount() {
    if (dialog) return;
    dialog = document.createElement("dialog"); dialog.id = "appearanceDialog"; dialog.className = "appearance-dialog";
    dialog.setAttribute("aria-labelledby", "appearanceTitle");
    const langButtons = [["uk", "Українська", "UA"], ["ru", "Русский", "RU"], ["en", "English", "EN"]].map(([code, name, short]) =>
      `<button type="button" class="appearance-seg-btn" role="radio" data-appearance-language="${code}" aria-label="${name}" title="${name}" aria-checked="false">${short}</button>`
    ).join("");
    const tintSwatches = tintPresets.map(item =>
      `<button type="button" class="appearance-tint-swatch${item.key ? "" : " is-none"}" role="radio" data-tint-preset="${item.key}" data-appearance-label="${item.label}" aria-checked="false"${item.color ? ` style="--swatch:${item.color}"` : ""}><span aria-hidden="true"></span></button>`
    ).join("");
    dialog.innerHTML = `
      <header class="appearance-head">
        <div class="appearance-head-top">
          <div><h2 id="appearanceTitle" data-appearance-text="title"></h2><p data-appearance-text="subtitle"></p></div>
          <button type="button" class="btn icon-only ghost" data-appearance-close data-appearance-label="close"><svg class="ico" aria-hidden="true"><use href="#i-x"/></svg></button>
        </div>
        <div class="appearance-tabs" role="tablist" data-seg-group>
          <span class="appearance-seg-thumb" aria-hidden="true"></span>
          <button type="button" class="appearance-tab" role="tab" data-appearance-tab="appearance" aria-selected="true" data-appearance-text="tabAppearance"></button>
          <button type="button" class="appearance-tab" role="tab" data-appearance-tab="workspace" aria-selected="false" data-appearance-text="tabWorkspace"></button>
          <button type="button" class="appearance-tab" role="tab" data-appearance-tab="app" aria-selected="false" data-appearance-text="tabApp"></button>
        </div>
      </header>
      <div class="appearance-content">
        <div class="appearance-panel is-active" data-appearance-panel="appearance">
          <fieldset class="appearance-section">
            <legend data-appearance-text="theme"></legend>
            <div class="appearance-themes-sub" data-appearance-text="classicThemes"></div>
            <div class="appearance-themes">${classicThemes.map(themeTile).join("")}</div>
            <div class="appearance-themes-sub appearance-themes-sub-cat" data-appearance-text="catThemes"></div>
            <div class="appearance-themes">${catThemes.map(themeTile).join("")}</div>
            <div class="appearance-themes-sub appearance-themes-sub-cat" data-appearance-text="manulThemes"></div>
            <div class="appearance-themes">${manulThemes.map(themeTile).join("")}</div>
          </fieldset>
          <div class="appearance-card">
            <div class="appearance-card-block">
              <div class="appearance-block-head"><strong data-appearance-text="surfaceTint"></strong><small data-appearance-text="surfaceTintNote"></small></div>
              <div class="appearance-accents appearance-tints" role="radiogroup">${tintSwatches}<button type="button" class="appearance-custom-accent appearance-custom-tint" data-color-open="surfaceTint" data-appearance-label="chooseColor" aria-haspopup="dialog" aria-expanded="false" aria-controls="appearanceColorPicker"><svg class="ico" aria-hidden="true"><use href="#i-sliders"/></svg></button></div>
            </div>
            ${effect("ambientGradient", "ambientGradientNote")}
          </div>
          <div class="appearance-card">
            <div class="appearance-card-block">
              <div class="appearance-block-head"><strong data-appearance-text="accent"></strong></div>
              <div class="appearance-accents">${Object.entries(accents).map(([name, a]) => `<label class="appearance-accent" style="--swatch:${a.dark}"><input type="radio" name="appearance-accent" data-pref="accent" value="${name}" data-appearance-label="${name}"><span aria-hidden="true"></span></label>`).join("")}<button type="button" class="appearance-custom-accent" data-color-open="accent" data-appearance-label="chooseColor" aria-haspopup="dialog" aria-expanded="false" aria-controls="appearanceColorPicker"><svg class="ico" aria-hidden="true"><use href="#i-sliders"/></svg></button></div>
            </div>
            <div class="appearance-row" role="group" aria-labelledby="lblPrimaryStyle"><span class="appearance-row-label" id="lblPrimaryStyle" data-appearance-text="primaryStyle"></span>${seg(radio("primaryStyle", "neutral") + radio("primaryStyle", "accent", "accentButtons"))}</div>
          </div>
          <details class="appearance-palette"><summary><span><strong data-appearance-text="customPalette"></strong><small data-appearance-text="paletteNote"></small></span><svg class="ico" aria-hidden="true"><use href="#i-chev-down"/></svg></summary><div class="appearance-colors">${[...colorKeys, "accent"].map(colorField).join("")}</div><div class="appearance-contrast" id="appearanceContrastNote" hidden><span data-appearance-text="contrastNote"></span><button type="button" data-appearance-fix-contrast data-appearance-text="fixContrast"></button></div><button type="button" class="appearance-reset appearance-reset-colors" data-appearance-reset-colors data-appearance-text="resetColors"></button></details>
          <section class="appearance-group"><h3 data-appearance-text="corners"></h3><div class="appearance-card">${range("controlRadius", "ControlRadius", 0, 16)}${range("panelRadius", "PanelRadius", 0, 20)}</div></section>
        </div>

        <div class="appearance-panel" data-appearance-panel="workspace" hidden>
          <section class="appearance-group appearance-group-first"><h3 data-appearance-text="toolsGroup"></h3><div class="appearance-card">${effect("showTokenCounter", "showTokenCounterNote")}${effect("showSlimTool", "showSlimToolNote")}</div></section>
          <section class="appearance-group"><h3 data-appearance-text="reading"></h3><div class="appearance-card"><div class="appearance-row" role="group" aria-labelledby="lblDensity"><span class="appearance-row-label" id="lblDensity" data-appearance-text="density"></span>${seg(radio("density", "comfortable") + radio("density", "compact"))}</div>${range("textSize", "TextSize", 12, 20)}${range("lineHeight", "LineHeight", 1.4, 2, .1)}<div class="appearance-row" role="group" aria-labelledby="lblWidth"><span class="appearance-row-label" id="lblWidth" data-appearance-text="width"></span>${seg(radio("readingWidth", "normal") + radio("readingWidth", "wide") + radio("readingWidth", "full"))}</div></div></section>
        </div>

        <div class="appearance-panel" data-appearance-panel="app" hidden>
          <section class="appearance-group appearance-group-first"><h3 data-appearance-text="general"></h3><div class="appearance-card"><div class="appearance-row appearance-language-row"><span class="appearance-row-label" id="appearanceLanguageLabel" data-appearance-text="language"></span><div class="appearance-options appearance-lang-seg" role="radiogroup" aria-labelledby="appearanceLanguageLabel" data-seg-group><span class="appearance-seg-thumb" aria-hidden="true"></span>${langButtons}</div></div><div class="appearance-row" role="group" aria-labelledby="lblMotion"><span class="appearance-row-label" id="lblMotion" data-appearance-text="motion"></span>${seg(radio("motion", "system", "motionSystem") + radio("motion", "full", "motionFull") + radio("motion", "reduced", "motionReduced"))}</div></div></section>
          <section class="appearance-group"><h3 data-appearance-text="effects"></h3><div class="appearance-card">${effect("snow", "snowNote")}${effect("cat", "catNote")}</div><p class="appearance-note appearance-effects-note" id="appearanceEffectsNote" data-appearance-text="effectsPaused" hidden></p></section>
        </div>
      </div>
      <footer class="appearance-foot"><div class="appearance-foot-copy"><button type="button" class="appearance-reset" data-appearance-reset data-appearance-text="reset"></button><span class="appearance-note" id="appearanceSaveNote" role="status"></span></div><button type="button" class="btn primary" data-appearance-close data-appearance-text="done"></button></footer>${pickerMarkup()}`;
    document.body.appendChild(dialog);
    picker = dialog.querySelector("#appearanceColorPicker");
    preferenceInputs = [...dialog.querySelectorAll("input[data-pref]")]; colorButtons = [...dialog.querySelectorAll("[data-color-open]")]; hexInputs = [...dialog.querySelectorAll(".appearance-hex")];
    textNodes = [...dialog.querySelectorAll("[data-appearance-text]")]; labelNodes = [...dialog.querySelectorAll("[data-appearance-label]")];
    tintButtons = [...dialog.querySelectorAll("[data-tint-preset]")];
    function inputPatch(input) {
      if (input.dataset.colorKey) {
        const color = hex(input.value); if (!color) return null;
        return input.dataset.colorKey === "accent" ? { accent: "custom", customAccent: color } : { customColors: { [input.dataset.colorKey]: color } };
      }
      if (!input.dataset.pref) return null;
      return { [input.dataset.pref]: input.type === "checkbox" ? input.checked : input.type === "range" ? Number(input.value) : input.value };
    }
    dialog.addEventListener("input", event => {
      const input = event.target;
      if (input.type === "range" || input.dataset.colorKey) { const patch = inputPatch(input); if (patch) queuePreview(patch); }
    });
    dialog.addEventListener("change", event => {
      const input = event.target, patch = inputPatch(input);
      if (!patch) { if (input.classList.contains("appearance-hex")) input.setAttribute("aria-invalid", "true"); return; }
      input.removeAttribute("aria-invalid"); settlePreview();
      if (patch.customColors) patch.customColors = { ...settings.customColors, ...patch.customColors };
      update(patch, { animate: input.type !== "range" && !input.dataset.colorKey });
      const grp = input.closest("[data-seg-group]");
      if (grp) raf(() => syncSegGroup(grp, false));
    });
    dialog.addEventListener("focusout", event => { if (event.target.classList.contains("appearance-hex")) { settlePreview(); event.target.removeAttribute("aria-invalid"); event.target.value = palette()[event.target.dataset.colorKey]; } });
    dialog.addEventListener("click", event => {
      const tab = event.target.closest("[data-appearance-tab]");
      const tint = event.target.closest("[data-tint-preset]");
      const color = event.target.closest("[data-color-open]");
      const lang = event.target.closest("[data-appearance-language]");
      if (tab) setTab(tab.dataset.appearanceTab, false);
      else if (tint) { settlePreview(); update({ surfaceTint: tint.dataset.tintPreset, customColors: {} }); }
      else if (color) openPicker(color);
      else if (lang) { settlePreview(); window.dispatchEvent(new CustomEvent("k1tty:languagechange", { detail: { language: lang.dataset.appearanceLanguage } })); }
      else if (event.target.closest("[data-appearance-close]")) close();
      else if (event.target.closest("[data-appearance-reset]")) { closePicker(false); settlePreview(); update({ ...defaults, customColors: {} }); }
      else if (event.target.closest("[data-appearance-reset-colors]")) { closePicker(false); settlePreview(); update({ customColors: {}, surfaceTint: "", accent: defaultAccentFor(settings.theme) }); }
      else if (event.target.closest("[data-appearance-fix-contrast]")) { const p = palette(); update({ customColors: { ...settings.customColors, text: readable(p.text, [p.canvas, p.sidebar, p.surface]) } }); }
      else if (event.target === dialog) { const rect = dialog.getBoundingClientRect(); if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) close(); }
    });
    dialog.addEventListener("keydown", event => {
      if (event.key === "Escape") {
        if (pickerState) { event.preventDefault(); event.stopPropagation(); closePicker(); return; }
        event.preventDefault(); event.stopPropagation(); close(); return;
      }
      const current = event.target.closest("[data-appearance-language]");
      if (current && ["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) {
        event.preventDefault();
        const buttons = [...dialog.querySelectorAll("[data-appearance-language]")], index = buttons.indexOf(current);
        const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowRight" ? 1 : -1) + buttons.length) % buttons.length;
        buttons[next].focus({ preventScroll: true }); buttons[next].click();
      }
      if (event.key !== "F11") event.stopPropagation();
    });
    dialog.addEventListener("cancel", event => { event.preventDefault(); if (pickerState) closePicker(); else close(); });
    dialog.addEventListener("pointerdown", event => { if (pickerState && !picker.contains(event.target) && !event.target.closest("[data-color-open]")) closePicker(); });
    dialog.addEventListener("focusin", event => { if (pickerState && !picker.contains(event.target) && !event.target.closest("[data-color-open]")) closePicker(false); });
    dialog.querySelector(".appearance-content").addEventListener("scroll", () => closePicker(), { passive: true });
    window.addEventListener("resize", () => { closePicker(); syncAllSegs(true); });
    const plane = picker.querySelector("#appearancePickerPlane");
    function movePointer(event) {
      if (!pickerState || !pickerPointer || event.pointerId !== pickerPointer.id) return;
      const rect = pickerPointer.rect;
      pickerState.hsv.s = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
      pickerState.hsv.v = Math.max(0, Math.min(1, 1 - (event.clientY - rect.top) / rect.height));
      previewPicker(fromHsv(pickerState.hsv), true);
    }
    plane.addEventListener("pointerdown", event => {
      if (!pickerState || pickerPointer || event.button !== 0) return;
      const rect = plane.getBoundingClientRect();
      if (!rect.width || !rect.height) return;
      event.preventDefault(); plane.focus({ preventScroll: true });
      pickerPointer = { id: event.pointerId, element: plane, rect };
      try { plane.setPointerCapture?.(event.pointerId); } catch {}
      movePointer(event);
    });
    plane.addEventListener("pointermove", movePointer);
    plane.addEventListener("pointerup", event => { if (pickerPointer?.id === event.pointerId) { movePointer(event); stopPickerPointer(); settlePreview(); } });
    plane.addEventListener("pointercancel", () => { stopPickerPointer(); settlePreview(); });
    plane.addEventListener("lostpointercapture", () => { pickerPointer = null; settlePreview(); });
    plane.addEventListener("keydown", event => {
      if (!pickerState || !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End", "PageUp", "PageDown"].includes(event.key)) return;
      event.preventDefault(); event.stopPropagation();
      const step = event.shiftKey ? .1 : .01, hsv = pickerState.hsv;
      if (event.key === "ArrowLeft") hsv.s -= step;
      if (event.key === "ArrowRight") hsv.s += step;
      if (event.key === "ArrowUp") hsv.v += step;
      if (event.key === "ArrowDown") hsv.v -= step;
      if (event.key === "Home") hsv.s = 0;
      if (event.key === "End") hsv.s = 1;
      if (event.key === "PageUp") hsv.v += .1;
      if (event.key === "PageDown") hsv.v -= .1;
      hsv.s = Math.max(0, Math.min(1, hsv.s)); hsv.v = Math.max(0, Math.min(1, hsv.v));
      previewPicker(fromHsv(hsv), true);
    });
    plane.addEventListener("keyup", () => settlePreview());
    picker.addEventListener("input", event => {
      if (!pickerState) return;
      const input = event.target;
      if (input.id === "appearancePickerHue") { pickerState.hsv.h = Number(input.value); previewPicker(fromHsv(pickerState.hsv), true); }
      else if (input.id === "appearancePickerHex") {
        const value = hex(input.value);
        if (value) { input.removeAttribute("aria-invalid"); previewPicker(value); }
        else input.setAttribute("aria-invalid", "true");
      } else if (input.dataset.pickerChannel) {
        const inputs = [...picker.querySelectorAll("[data-picker-channel]")];
        const channels = inputs.map(field => Number(field.value));
        const valid = inputs.every((field, index) => field.value.trim() !== "" && Number.isInteger(channels[index]) && channels[index] >= 0 && channels[index] <= 255);
        if (!valid) input.setAttribute("aria-invalid", "true");
        if (valid) { input.removeAttribute("aria-invalid"); previewPicker(fromRgb(channels)); }
      }
    });
    function settlePickerInput(input) {
      if (!pickerState) return;
      settlePreview(); input.removeAttribute("aria-invalid");
      if (input.id === "appearancePickerHex") input.value = pickerState.value;
      else if (input.dataset.pickerChannel) input.value = String(rgb(pickerState.value)[["r", "g", "b"].indexOf(input.dataset.pickerChannel)]);
      drawPicker();
    }
    picker.addEventListener("change", event => settlePickerInput(event.target));
    picker.addEventListener("focusout", event => { if (event.target.matches("input")) settlePickerInput(event.target); });
    picker.addEventListener("click", event => {
      if (!pickerState) return;
      if (event.target.closest("[data-picker-close]")) { closePicker(); return; }
      const swatch = event.target.closest("[data-picker-swatch]");
      if (swatch) { previewPicker(swatch.dataset.pickerSwatch); settlePreview(); }
      else if (event.target.closest("[data-picker-previous]")) { previewPicker(pickerState.previous); settlePreview(); }
    });
    dialog.addEventListener("close", () => {
      dialog.classList.remove("is-closing");
      closePicker(false); settlePreview();
      if (opener?.isConnected) opener.focus({ preventScroll: true });
    });
    document.getElementById("appearanceBtn")?.addEventListener("click", () => open());
    new MutationObserver(() => setLanguage(root.lang)).observe(root, { attributes: true, attributeFilter: ["lang"] });
    setLanguage(root.lang); apply();
  }
  function open(tab) {
    mount();
    window.clearTimeout(closeTimer); closeTimer = 0;
    dialog.classList.remove("is-closing");
    if (!dialog.open) {
      opener = document.activeElement;
      syncControls();
      dialog.showModal();
    }
    const nextTab = typeof tab === "string" && ["appearance", "workspace", "app"].includes(tab) ? tab : activeTab;
    setTab(nextTab, true);
    raf(() => raf(() => syncAllSegs(true)));
  }
  function close() {
    closePicker(false); settlePreview();
    if (!dialog?.open) return;
    if (isReducedMotion()) { dialog.close(); return; }
    dialog.classList.add("is-closing");
    window.clearTimeout(closeTimer);
    closeTimer = window.setTimeout(() => { closeTimer = 0; if (dialog.open) dialog.close(); }, 150);
  }
  function getThemeLabel(theme, lang) {
    const dict = words[lang] || words[language] || words.en;
    return dict[theme] || theme;
  }
  mediaReduced?.addEventListener?.("change", () => { apply(); emit(); });
  window.addEventListener("pagehide", settlePreview);
  document.addEventListener("visibilitychange", () => { if (document.visibilityState === "hidden") settlePreview(); });
  window.addEventListener("storage", event => {
    if (event.key !== KEY) return;
    try { const external = normalize(JSON.parse(event.newValue || "null")); if (previewFrame) cancelRaf?.(previewFrame); previewFrame = 0; previewPatch = null; window.clearTimeout(saveTimer); saveTimer = 0; settings = external; dirty = false; apply({ animate: true }); emit(); } catch {}
  });
  window.K1ttyAppearance = Object.freeze({
    open,
    close,
    update,
    getSettings: copySettings,
    setLanguage,
    isReducedMotion,
    renderCatSvg,
    getThemeLabel,
    classicThemes: Object.freeze([...classicThemes]),
    catThemes: Object.freeze([...catThemes]),
    manulThemes: Object.freeze([...manulThemes]),
    allThemes: Object.freeze([...classicThemes, ...catThemes, ...manulThemes]),
  });
  apply();
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", mount, { once: true }); else mount();
})();
