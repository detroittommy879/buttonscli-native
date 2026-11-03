// Site structure shared by astro.config.mjs, the docs sync step and components.
// Pages come from docs/guide/*.md; any page not listed here lands in "More".

export const REPO_URL = 'https://github.com/detroittommy879/buttonscli-native';

/** Sidebar groups, in order. `section` picks the accent in looks that color sections. */
export const GROUPS = [
	{ key: 'start', section: 'start', pages: ['index', 'getting-started', 'workspace'] },
	{ key: 'looks', section: 'themes', pages: ['themes', 'theme-bundles', 'theme-gallery'] },
	{ key: 'shaders', section: 'shaders', pages: ['shaders', 'shader-code'] },
	{ key: 'ai', section: 'ai', pages: ['ai-setup', 'ai-shaders', 'ai-themes'] },
	{ key: 'reference', section: 'reference', pages: ['sharing', 'troubleshooting'] },
];

/** Guide files that are for contributors, not readers. */
export const EXCLUDED_PAGES = ['IMAGES'];

/** Pages whose accent differs from their group's. */
const PAGE_SECTIONS = { workspace: 'workspace' };

export function sectionFor(slug) {
	const page = slug || 'index';
	const group = GROUPS.find((g) => g.pages.includes(page));
	return PAGE_SECTIONS[page] ?? group?.section ?? 'reference';
}

/**
 * Every language the app ships. A language gets pages on the site once
 * docs/guide/i18n/<code>/ exists (or with DOCS_ALL_LOCALES=1).
 * `code` is the app's locale code and the folder name; URLs use it lowercased.
 */
export const LANGUAGES = [
	{ code: 'es', lang: 'es', label: 'Español' },
	{ code: 'de', lang: 'de', label: 'Deutsch' },
	{ code: 'fr', lang: 'fr', label: 'Français' },
	{ code: 'ja', lang: 'ja', label: '日本語' },
	{ code: 'pt-BR', lang: 'pt-BR', label: 'Português (Brasil)' },
	{ code: 'zh-CN', lang: 'zh-CN', label: '简体中文' },
	{ code: 'zh-TW', lang: 'zh-TW', label: '繁體中文' },
	{ code: 'ko', lang: 'ko', label: '한국어' },
	{ code: 'it', lang: 'it', label: 'Italiano' },
	{ code: 'ru', lang: 'ru', label: 'Русский' },
	{ code: 'uk', lang: 'uk', label: 'Українська' },
	{ code: 'pl', lang: 'pl', label: 'Polski' },
	{ code: 'nl', lang: 'nl', label: 'Nederlands' },
	{ code: 'sv', lang: 'sv', label: 'Svenska' },
	{ code: 'da', lang: 'da', label: 'Dansk' },
	{ code: 'no', lang: 'nb', label: 'Norsk' },
	{ code: 'fi', lang: 'fi', label: 'Suomi' },
	{ code: 'tr', lang: 'tr', label: 'Türkçe' },
	{ code: 'hi', lang: 'hi', label: 'हिन्दी' },
	{ code: 'ar', lang: 'ar', label: 'العربية', dir: 'rtl' },
];

/**
 * Site-only strings (Starlight translates its own UI). Missing keys fall back
 * to English. Group labels match the app's own section names where it has one.
 */
export const UI = {
	en: {
		appearance: 'Appearance', mode: 'Mode', look: 'Look', accent: 'Accent', auto: 'Auto',
		getStarted: 'Get started', screenshotWanted: 'Screenshot wanted',
		machineTranslated: 'This page was translated by AI from the English original. Corrections are welcome.',
		outdated: 'The English page has changed since this translation was made.',
		readEnglish: 'Read the English page',
		start: 'Start here', workspace: 'Workspace', looks: 'Themes', shaders: 'Shaders', ai: 'AI', reference: 'Reference', more: 'More',
	},
	es: {
		appearance: 'Apariencia', mode: 'Modo', look: 'Estilo', accent: 'Color', auto: 'Auto',
		getStarted: 'Empezar', screenshotWanted: 'Falta captura',
		machineTranslated: 'Esta página fue traducida por IA desde el original en inglés. Se agradecen correcciones.',
		outdated: 'La página en inglés cambió después de esta traducción.',
		readEnglish: 'Leer la página en inglés',
		start: 'Primeros pasos', workspace: 'Espacio de trabajo', looks: 'Temas', shaders: 'Shaders', ai: 'IA', reference: 'Referencia', more: 'Más',
	},
	de: {
		appearance: 'Darstellung', mode: 'Modus', look: 'Stil', accent: 'Akzent', auto: 'Auto',
		getStarted: 'Loslegen', screenshotWanted: 'Screenshot fehlt',
		machineTranslated: 'Diese Seite wurde von KI aus dem englischen Original übersetzt. Korrekturen sind willkommen.',
		outdated: 'Die englische Seite wurde seit dieser Übersetzung geändert.',
		readEnglish: 'Englische Seite lesen',
		start: 'Erste Schritte', workspace: 'Arbeitsbereich', looks: 'Themes', shaders: 'Shader', ai: 'KI', reference: 'Referenz', more: 'Mehr',
	},
	fr: {
		appearance: 'Apparence', mode: 'Mode', look: 'Style', accent: 'Accent', auto: 'Auto',
		getStarted: 'Commencer', screenshotWanted: 'Capture à faire',
		machineTranslated: 'Cette page a été traduite par IA depuis l’original anglais. Les corrections sont bienvenues.',
		outdated: 'La page anglaise a changé depuis cette traduction.',
		readEnglish: 'Lire la page en anglais',
		start: 'Pour commencer', workspace: 'Espace de travail', looks: 'Thèmes', shaders: 'Shaders', ai: 'IA', reference: 'Référence', more: 'Plus',
	},
	ja: {
		appearance: '外観', mode: 'モード', look: 'スタイル', accent: 'アクセント', auto: '自動',
		getStarted: 'はじめる', screenshotWanted: 'スクリーンショット未作成',
		machineTranslated: 'このページは英語版から AI で翻訳されました。修正を歓迎します。',
		outdated: 'この翻訳の後に英語版のページが更新されています。',
		readEnglish: '英語版を読む',
		start: 'はじめに', workspace: 'ワークスペース', looks: 'テーマ', shaders: 'シェーダー', ai: 'AI', reference: 'リファレンス', more: 'その他',
	},
	'pt-BR': {
		appearance: 'Aparência', mode: 'Modo', look: 'Estilo', accent: 'Destaque', auto: 'Auto',
		getStarted: 'Começar', screenshotWanted: 'Captura pendente',
		machineTranslated: 'Esta página foi traduzida por IA a partir do original em inglês. Correções são bem-vindas.',
		outdated: 'A página em inglês mudou depois desta tradução.',
		readEnglish: 'Ler a página em inglês',
		start: 'Comece aqui', workspace: 'Área de trabalho', looks: 'Temas', shaders: 'Shaders', ai: 'IA', reference: 'Referência', more: 'Mais',
	},
	'zh-CN': {
		appearance: '外观', mode: '模式', look: '风格', accent: '强调色', auto: '自动',
		getStarted: '开始使用', screenshotWanted: '待补截图',
		machineTranslated: '本页由 AI 从英文原文翻译，欢迎指正。',
		outdated: '英文页面在此翻译之后有更新。',
		readEnglish: '阅读英文页面',
		start: '入门', workspace: '工作区', looks: '主题', shaders: '着色器', ai: 'AI', reference: '参考', more: '更多',
	},
	'zh-TW': {
		appearance: '外觀', mode: '模式', look: '風格', accent: '強調色', auto: '自動',
		getStarted: '開始使用', screenshotWanted: '待補截圖',
		machineTranslated: '本頁由 AI 從英文原文翻譯，歡迎指正。',
		outdated: '英文頁面在此翻譯之後有更新。',
		readEnglish: '閱讀英文頁面',
		start: '入門', workspace: '工作區', looks: '主題', shaders: '著色器', ai: 'AI', reference: '參考', more: '更多',
	},
	ko: {
		appearance: '모양', mode: '모드', look: '스타일', accent: '강조색', auto: '자동',
		getStarted: '시작하기', screenshotWanted: '스크린샷 필요',
		machineTranslated: '이 페이지는 영어 원문을 AI로 번역했습니다. 수정 제안을 환영합니다.',
		outdated: '이 번역 이후 영어 페이지가 변경되었습니다.',
		readEnglish: '영어 페이지 읽기',
		start: '시작하기', workspace: '작업 공간', looks: '테마', shaders: '셰이더', ai: 'AI', reference: '참고', more: '더 보기',
	},
	it: {
		appearance: 'Aspetto', mode: 'Modalità', look: 'Stile', accent: 'Colore', auto: 'Auto',
		getStarted: 'Inizia', screenshotWanted: 'Screenshot mancante',
		machineTranslated: 'Questa pagina è stata tradotta dall’IA dall’originale inglese. Le correzioni sono benvenute.',
		outdated: 'La pagina inglese è cambiata dopo questa traduzione.',
		readEnglish: 'Leggi la pagina in inglese',
		start: 'Per iniziare', workspace: 'Area di lavoro', looks: 'Temi', shaders: 'Shader', ai: 'IA', reference: 'Riferimento', more: 'Altro',
	},
	ru: {
		appearance: 'Оформление', mode: 'Режим', look: 'Стиль', accent: 'Акцент', auto: 'Авто',
		getStarted: 'Начать', screenshotWanted: 'Нужен скриншот',
		machineTranslated: 'Эта страница переведена ИИ с английского оригинала. Исправления приветствуются.',
		outdated: 'Английская страница изменилась после этого перевода.',
		readEnglish: 'Читать на английском',
		start: 'Начало', workspace: 'Рабочая область', looks: 'Темы', shaders: 'Шейдеры', ai: 'ИИ', reference: 'Справка', more: 'Ещё',
	},
	uk: {
		appearance: 'Вигляд', mode: 'Режим', look: 'Стиль', accent: 'Акцент', auto: 'Авто',
		getStarted: 'Почати', screenshotWanted: 'Потрібен знімок',
		machineTranslated: 'Цю сторінку перекладено ШІ з англійського оригіналу. Виправлення вітаються.',
		outdated: 'Англійська сторінка змінилася після цього перекладу.',
		readEnglish: 'Читати англійською',
		start: 'Початок', workspace: 'Робоча область', looks: 'Теми', shaders: 'Шейдери', ai: 'ШІ', reference: 'Довідка', more: 'Більше',
	},
	pl: {
		appearance: 'Wygląd', mode: 'Tryb', look: 'Styl', accent: 'Akcent', auto: 'Auto',
		getStarted: 'Zacznij', screenshotWanted: 'Brak zrzutu',
		machineTranslated: 'Ta strona została przetłumaczona przez AI z angielskiego oryginału. Poprawki mile widziane.',
		outdated: 'Angielska strona zmieniła się po tym tłumaczeniu.',
		readEnglish: 'Czytaj po angielsku',
		start: 'Na początek', workspace: 'Obszar roboczy', looks: 'Motywy', shaders: 'Shadery', ai: 'AI', reference: 'Dokumentacja', more: 'Więcej',
	},
	nl: {
		appearance: 'Weergave', mode: 'Modus', look: 'Stijl', accent: 'Accent', auto: 'Auto',
		getStarted: 'Aan de slag', screenshotWanted: 'Screenshot nodig',
		machineTranslated: 'Deze pagina is door AI vertaald uit het Engelse origineel. Verbeteringen zijn welkom.',
		outdated: 'De Engelse pagina is gewijzigd sinds deze vertaling.',
		readEnglish: 'Lees de Engelse pagina',
		start: 'Begin hier', workspace: 'Werkruimte', looks: "Thema's", shaders: 'Shaders', ai: 'AI', reference: 'Naslag', more: 'Meer',
	},
	sv: {
		appearance: 'Utseende', mode: 'Läge', look: 'Stil', accent: 'Accent', auto: 'Auto',
		getStarted: 'Kom igång', screenshotWanted: 'Skärmbild saknas',
		machineTranslated: 'Sidan är översatt av AI från det engelska originalet. Rättelser välkomnas.',
		outdated: 'Den engelska sidan har ändrats sedan översättningen.',
		readEnglish: 'Läs den engelska sidan',
		start: 'Börja här', workspace: 'Arbetsyta', looks: 'Teman', shaders: 'Shaders', ai: 'AI', reference: 'Referens', more: 'Mer',
	},
	da: {
		appearance: 'Udseende', mode: 'Tilstand', look: 'Stil', accent: 'Accent', auto: 'Auto',
		getStarted: 'Kom i gang', screenshotWanted: 'Skærmbillede mangler',
		machineTranslated: 'Siden er oversat af AI fra den engelske original. Rettelser er velkomne.',
		outdated: 'Den engelske side er ændret siden oversættelsen.',
		readEnglish: 'Læs den engelske side',
		start: 'Start her', workspace: 'Arbejdsområde', looks: 'Temaer', shaders: 'Shaders', ai: 'AI', reference: 'Reference', more: 'Mere',
	},
	no: {
		appearance: 'Utseende', mode: 'Modus', look: 'Stil', accent: 'Aksent', auto: 'Auto',
		getStarted: 'Kom i gang', screenshotWanted: 'Skjermbilde mangler',
		machineTranslated: 'Siden er oversatt av KI fra den engelske originalen. Rettelser er velkomne.',
		outdated: 'Den engelske siden er endret etter oversettelsen.',
		readEnglish: 'Les den engelske siden',
		start: 'Start her', workspace: 'Arbeidsområde', looks: 'Temaer', shaders: 'Shadere', ai: 'KI', reference: 'Referanse', more: 'Mer',
	},
	fi: {
		appearance: 'Ulkoasu', mode: 'Tila', look: 'Tyyli', accent: 'Korostus', auto: 'Auto',
		getStarted: 'Aloita', screenshotWanted: 'Kuvakaappaus puuttuu',
		machineTranslated: 'Tämä sivu on tekoälyn kääntämä englanninkielisestä alkuperäisestä. Korjaukset ovat tervetulleita.',
		outdated: 'Englanninkielinen sivu on muuttunut käännöksen jälkeen.',
		readEnglish: 'Lue englanninkielinen sivu',
		start: 'Aloita tästä', workspace: 'Työtila', looks: 'Teemat', shaders: 'Varjostimet', ai: 'Tekoäly', reference: 'Viite', more: 'Lisää',
	},
	tr: {
		appearance: 'Görünüm', mode: 'Mod', look: 'Stil', accent: 'Vurgu', auto: 'Otomatik',
		getStarted: 'Başlayın', screenshotWanted: 'Ekran görüntüsü gerekli',
		machineTranslated: 'Bu sayfa İngilizce asıldan yapay zekâ ile çevrildi. Düzeltmeler memnuniyetle karşılanır.',
		outdated: 'İngilizce sayfa bu çeviriden sonra değişti.',
		readEnglish: 'İngilizce sayfayı oku',
		start: 'Buradan başlayın', workspace: 'Çalışma alanı', looks: 'Temalar', shaders: 'Gölgelendiriciler', ai: 'Yapay zekâ', reference: 'Başvuru', more: 'Daha fazla',
	},
	hi: {
		appearance: 'रूप', mode: 'मोड', look: 'शैली', accent: 'एक्सेंट', auto: 'ऑटो',
		getStarted: 'शुरू करें', screenshotWanted: 'स्क्रीनशॉट चाहिए',
		machineTranslated: 'यह पेज अंग्रेज़ी मूल से AI द्वारा अनुवादित है। सुधार का स्वागत है।',
		outdated: 'इस अनुवाद के बाद अंग्रेज़ी पेज बदल गया है।',
		readEnglish: 'अंग्रेज़ी पेज पढ़ें',
		start: 'यहाँ से शुरू करें', workspace: 'वर्कस्पेस', looks: 'थीम', shaders: 'शेडर', ai: 'AI', reference: 'संदर्भ', more: 'और',
	},
	ar: {
		appearance: 'المظهر', mode: 'الوضع', look: 'النمط', accent: 'لون التمييز', auto: 'تلقائي',
		getStarted: 'ابدأ', screenshotWanted: 'لقطة شاشة مطلوبة',
		machineTranslated: 'تُرجمت هذه الصفحة بالذكاء الاصطناعي من الأصل الإنجليزي. نرحب بالتصحيحات.',
		outdated: 'تغيّرت الصفحة الإنجليزية بعد هذه الترجمة.',
		readEnglish: 'اقرأ الصفحة الإنجليزية',
		start: 'ابدأ هنا', workspace: 'مساحة العمل', looks: 'السمات', shaders: 'المظللات', ai: 'الذكاء الاصطناعي', reference: 'مرجع', more: 'المزيد',
	},
};

/** Translate a site string for a BCP-47 tag or app locale code. */
export function ui(langOrCode, key) {
	const code = LANGUAGES.find((l) => l.lang === langOrCode || l.code === langOrCode)?.code;
	return (code && UI[code]?.[key]) || UI.en[key] || key;
}

/** Looks you can switch between at runtime. Each has dark and light colors in looks.css. */
export const LOOKS = [
	{ id: 'classic', label: 'Classic', sample: 'Inter' },
	{ id: 'neon', label: 'Neon', sample: 'Outfit' },
	{ id: 'paper', label: 'Paper', sample: 'Source Serif' },
	{ id: 'terminal', label: 'Terminal', sample: 'Plex Mono' },
];

/** Accent hues. `auto` uses the look's own accent (Neon colors each section). */
export const ACCENTS = [
	{ id: 'auto', hue: null },
	{ id: 'cyan', hue: 190 },
	{ id: 'violet', hue: 262 },
	{ id: 'green', hue: 150 },
	{ id: 'amber', hue: 40 },
	{ id: 'rose', hue: 345 },
	{ id: 'blue', hue: 222 },
];

export const DEFAULT_MODE = process.env.DOCS_DEFAULT_MODE || 'dark';
export const DEFAULT_LOOK = process.env.DOCS_DEFAULT_LOOK || 'classic';
