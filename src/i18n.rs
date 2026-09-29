//! Native UI localization backed by the imported source catalog, with English fallback.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

pub const SUPPORTED_LOCALES: [&str; 21] = [
    "en", "es", "zh-CN", "fr", "ja", "hi", "de", "pt-BR", "it", "ru", "uk", "ko", "ar", "tr", "pl",
    "nl", "sv", "da", "fi", "no", "zh-TW",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocaleInfo {
    pub code: &'static str,
    pub english_name: &'static str,
    pub native_name: &'static str,
    pub right_to_left: bool,
}

pub const LOCALE_INFO: [LocaleInfo; 21] = [
    LocaleInfo {
        code: "en",
        english_name: "English",
        native_name: "English",
        right_to_left: false,
    },
    LocaleInfo {
        code: "es",
        english_name: "Spanish",
        native_name: "Español",
        right_to_left: false,
    },
    LocaleInfo {
        code: "zh-CN",
        english_name: "Chinese (Simplified)",
        native_name: "简体中文",
        right_to_left: false,
    },
    LocaleInfo {
        code: "fr",
        english_name: "French",
        native_name: "Français",
        right_to_left: false,
    },
    LocaleInfo {
        code: "ja",
        english_name: "Japanese",
        native_name: "日本語",
        right_to_left: false,
    },
    LocaleInfo {
        code: "hi",
        english_name: "Hindi",
        native_name: "हिन्दी",
        right_to_left: false,
    },
    LocaleInfo {
        code: "de",
        english_name: "German",
        native_name: "Deutsch",
        right_to_left: false,
    },
    LocaleInfo {
        code: "pt-BR",
        english_name: "Portuguese (Brazil)",
        native_name: "Português (Brasil)",
        right_to_left: false,
    },
    LocaleInfo {
        code: "it",
        english_name: "Italian",
        native_name: "Italiano",
        right_to_left: false,
    },
    LocaleInfo {
        code: "ru",
        english_name: "Russian",
        native_name: "Русский",
        right_to_left: false,
    },
    LocaleInfo {
        code: "uk",
        english_name: "Ukrainian",
        native_name: "Українська",
        right_to_left: false,
    },
    LocaleInfo {
        code: "ko",
        english_name: "Korean",
        native_name: "한국어",
        right_to_left: false,
    },
    LocaleInfo {
        code: "ar",
        english_name: "Arabic",
        native_name: "العربية",
        right_to_left: true,
    },
    LocaleInfo {
        code: "tr",
        english_name: "Turkish",
        native_name: "Türkçe",
        right_to_left: false,
    },
    LocaleInfo {
        code: "pl",
        english_name: "Polish",
        native_name: "Polski",
        right_to_left: false,
    },
    LocaleInfo {
        code: "nl",
        english_name: "Dutch",
        native_name: "Nederlands",
        right_to_left: false,
    },
    LocaleInfo {
        code: "sv",
        english_name: "Swedish",
        native_name: "Svenska",
        right_to_left: false,
    },
    LocaleInfo {
        code: "da",
        english_name: "Danish",
        native_name: "Dansk",
        right_to_left: false,
    },
    LocaleInfo {
        code: "fi",
        english_name: "Finnish",
        native_name: "Suomi",
        right_to_left: false,
    },
    LocaleInfo {
        code: "no",
        english_name: "Norwegian",
        native_name: "Norsk",
        right_to_left: false,
    },
    LocaleInfo {
        code: "zh-TW",
        english_name: "Chinese (Traditional)",
        native_name: "繁體中文",
        right_to_left: false,
    },
];

#[derive(Deserialize)]
struct LiteralCatalog {
    messages: HashMap<String, HashMap<String, String>>,
}

static LITERAL_CATALOG: OnceLock<LiteralCatalog> = OnceLock::new();

fn literal_catalog() -> &'static LiteralCatalog {
    LITERAL_CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("i18n/literals.json"))
            .expect("bundled native literal catalog is valid JSON")
    })
}

pub fn literal(locale: &str, english: &str) -> String {
    let locale = resolve_locale(locale);
    if locale == "en" {
        return english.to_owned();
    }
    // The imported source catalog leaves this account action untranslated in three locales.
    if english == "Sign out" {
        let corrected = match locale {
            "fr" => Some("Se déconnecter"),
            "it" => Some("Disconnetti"),
            "zh-TW" => Some("登出"),
            _ => None,
        };
        if let Some(corrected) = corrected {
            return corrected.to_owned();
        }
    }
    if english == "Sign-in manages your ButtonsCLI account session and checks server access. Local terminals and provider settings work without an account." {
        let translated = match locale {
            "es" => "Iniciar sesión administra tu cuenta de ButtonsCLI y consulta los permisos del servidor. Los terminales locales y la configuración de proveedores funcionan sin una cuenta.",
            "ja" => "サインインすると ButtonsCLI アカウントのセッションを管理し、サーバーのアクセス権を確認します。ローカル端末とプロバイダー設定はアカウントなしでも利用できます。",
            "pt-BR" => "O login gerencia sua sessão da conta ButtonsCLI e verifica o acesso ao servidor. Os terminais locais e as configurações de provedores funcionam sem uma conta.",
            "zh-CN" => "登录会管理你的 ButtonsCLI 账户会话并检查服务器访问权限。没有账户也可以使用本地终端和提供商设置。",
            "fr" => "La connexion gère votre session ButtonsCLI et vérifie les autorisations du serveur. Les terminaux locaux et les paramètres des fournisseurs fonctionnent sans compte.",
            "hi" => "साइन इन करने से आपका ButtonsCLI खाता सत्र प्रबंधित होता है और सर्वर की पहुँच जाँची जाती है। स्थानीय टर्मिनल और प्रदाता सेटिंग बिना खाते के भी काम करते हैं।",
            "de" => "Die Anmeldung verwaltet deine ButtonsCLI-Kontositzung und prüft den Serverzugriff. Lokale Terminals und Anbietereinstellungen funktionieren auch ohne Konto.",
            "it" => "L'accesso gestisce la sessione del tuo account ButtonsCLI e verifica i permessi del server. I terminali locali e le impostazioni dei provider funzionano anche senza un account.",
            "ru" => "Вход управляет сеансом вашей учётной записи ButtonsCLI и проверяет доступ к серверу. Локальные терминалы и настройки провайдеров работают и без учётной записи.",
            "ko" => "로그인하면 ButtonsCLI 계정 세션을 관리하고 서버 접근 권한을 확인합니다. 로컬 터미널과 공급자 설정은 계정 없이도 사용할 수 있습니다.",
            "ar" => "تدير عملية تسجيل الدخول جلسة حساب ButtonsCLI وتتحقق من صلاحيات الوصول إلى الخادم. تعمل الطرفيات المحلية وإعدادات المزوّدين دون حساب.",
            "tr" => "Oturum açmak ButtonsCLI hesabı oturumunuzu yönetir ve sunucu erişimini denetler. Yerel terminaller ve sağlayıcı ayarları hesap olmadan da çalışır.",
            "pl" => "Logowanie zarządza sesją konta ButtonsCLI i sprawdza dostęp do serwera. Lokalne terminale i ustawienia dostawców działają bez konta.",
            "nl" => "Aanmelden beheert je ButtonsCLI-accountsessie en controleert de servertoegang. Lokale terminals en providerinstellingen werken zonder account.",
            "sv" => "Inloggning hanterar din ButtonsCLI-kontosession och kontrollerar serveråtkomst. Lokala terminaler och leverantörsinställningar fungerar utan konto.",
            "da" => "Log ind administrerer din ButtonsCLI-kontosession og kontrollerer serveradgang. Lokale terminaler og udbyderindstillinger fungerer uden en konto.",
            "fi" => "Kirjautuminen hallitsee ButtonsCLI-tilisi istuntoa ja tarkistaa palvelinkäyttöoikeudet. Paikalliset päätteet ja palveluasetukset toimivat ilman tiliä.",
            "no" => "Innlogging administrerer ButtonsCLI-kontoøkten din og kontrollerer servertillgang. Lokale terminaler og leverandørinnstillinger fungerer uten en konto.",
            "zh-TW" => "登入會管理你的 ButtonsCLI 帳戶工作階段並檢查伺服器存取權限。沒有帳戶也能使用本機終端機和提供者設定。",
            "uk" => "Вхід керує сеансом вашого облікового запису ButtonsCLI та перевіряє доступ до сервера. Локальні термінали й налаштування провайдерів працюють без облікового запису.",
            _ => english,
        };
        return translated.to_owned();
    }
    if let Some(translated) = quick_secrets_translation(locale, english) {
        return translated.to_owned();
    }
    let source_key = catalog_alias(english);
    let translations = literal_catalog().messages.get(source_key).or_else(|| {
        let mut matches = literal_catalog()
            .messages
            .iter()
            .filter(|(source, _)| source.eq_ignore_ascii_case(source_key));
        let first = matches.next()?.1;
        matches.next().is_none().then_some(first)
    });
    translations
        .and_then(|translations| translations.get(locale))
        .cloned()
        .unwrap_or_else(|| english.to_owned())
}

fn quick_secrets_translation(locale: &str, english: &str) -> Option<&'static str> {
    let value = match (english, locale) {
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "es",
        ) => "No hay recuperación. Si olvidas esta frase de contraseña, elimina la bóveda y crea otra.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "zh-CN",
        ) => "无法恢复。如果忘记此密码短语，请删除保险库并重新创建。",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "fr",
        ) => "Aucune récupération. Si vous oubliez cette phrase secrète, supprimez le coffre et créez-en un nouveau.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "ja",
        ) => "復旧手段はありません。このパスフレーズを忘れた場合は、保管庫を削除して新しく作成してください。",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "hi",
        ) => "इसे पुनर्प्राप्त नहीं किया जा सकता। पासफ़्रेज़ भूलने पर वॉल्ट हटाकर नया बनाएँ।",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "de",
        ) => "Keine Wiederherstellung möglich. Wenn du diese Passphrase vergisst, lösche den Tresor und erstelle einen neuen.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "pt-BR",
        ) => "Não há recuperação. Se você esquecer esta frase secreta, exclua o cofre e crie outro.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "it",
        ) => "Non è possibile recuperarla. Se dimentichi la passphrase, elimina il vault e creane uno nuovo.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "ru",
        ) => "Восстановление невозможно. Если вы забудете эту парольную фразу, удалите хранилище и создайте новое.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "uk",
        ) => "Відновлення неможливе. Якщо забудете цю парольну фразу, видаліть сховище та створіть нове.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "ko",
        ) => "복구할 수 없습니다. 이 암호 문구를 잊으면 보관함을 삭제하고 새로 만들어야 합니다.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "ar",
        ) => "لا يمكن الاسترداد. إذا نسيت عبارة المرور، فاحذف الخزنة وأنشئ واحدة جديدة.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "tr",
        ) => "Kurtarma yok. Bu parolayı unutursanız kasayı silip yenisini oluşturun.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "pl",
        ) => "Nie można odzyskać dostępu. Jeśli zapomnisz hasła, usuń sejf i utwórz nowy.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "nl",
        ) => "Herstel is niet mogelijk. Als je deze wachtwoordzin vergeet, verwijder dan de kluis en maak een nieuwe.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "sv",
        ) => "Det går inte att återställa. Om du glömmer lösenfrasen måste du ta bort valvet och skapa ett nytt.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "da",
        ) => "Adgang kan ikke gendannes. Hvis du glemmer adgangsfrasen, skal du slette boksen og oprette en ny.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "fi",
        ) => "Palautusta ei ole. Jos unohdat tunnuslauseen, poista holvi ja luo uusi.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "no",
        ) => "Gjenoppretting er ikke mulig. Hvis du glemmer passordfrasen, må du slette hvelvet og opprette et nytt.",
        (
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "zh-TW",
        ) => "無法復原。如果忘記此密語，請刪除保險庫並重新建立。",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "es",
        ) => "Elige un terminal listo. El shell puede mostrar el texto pegado en la salida del terminal.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "zh-CN",
        ) => "请明确选择一个就绪的终端。Shell 可能会将粘贴的文本回显到终端输出中。",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "fr",
        ) => "Choisissez un terminal prêt. Le shell peut afficher le texte collé dans sa sortie.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "ja",
        ) => "使用可能な端末を選択してください。シェルが貼り付けたテキストを端末出力に表示する場合があります。",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "hi",
        ) => "तैयार टर्मिनल चुनें। शेल चिपकाए गए पाठ को टर्मिनल आउटपुट में दिखा सकता है।",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "de",
        ) => "Wähle ein bereites Terminal. Die Shell kann eingefügten Text in der Terminalausgabe anzeigen.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "pt-BR",
        ) => "Escolha um terminal pronto. O shell pode exibir o texto colado na saída do terminal.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "it",
        ) => "Scegli un terminale pronto. La shell potrebbe mostrare il testo incollato nell'output del terminale.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "ru",
        ) => "Выберите готовый терминал. Оболочка может отобразить вставленный текст в выводе терминала.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "uk",
        ) => "Виберіть готовий термінал. Оболонка може показати вставлений текст у виводі термінала.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "ko",
        ) => "사용 가능한 터미널을 선택하세요. 셸이 붙여넣은 텍스트를 터미널 출력에 표시할 수 있습니다.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "ar",
        ) => "اختر طرفية جاهزة. قد تعرض الصدفة النص الملصق في مخرجات الطرفية.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "tr",
        ) => "Hazır bir terminal seçin. Kabuk, yapıştırılan metni terminal çıktısında gösterebilir.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "pl",
        ) => "Wybierz gotowy terminal. Powłoka może wyświetlić wklejony tekst w swoim wyjściu.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "nl",
        ) => "Kies een gereed terminal. De shell kan geplakte tekst in de terminaluitvoer tonen.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "sv",
        ) => "Välj en klar terminal. Skalet kan visa inklistrad text i terminalutdata.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "da",
        ) => "Vælg en klar terminal. Skallen kan vise indsat tekst i terminalens output.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "fi",
        ) => "Valitse valmis pääte. Komentotulkki voi näyttää liitetyn tekstin päätteen tulosteessa.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "no",
        ) => "Velg en klar terminal. Skallet kan vise innlimt tekst i terminalutdata.",
        (
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "zh-TW",
        ) => "請選擇可用的終端機。Shell 可能會將貼上的文字回顯到終端機輸出。",
        ("Forget vault", "es") => "Olvidar bóveda",
        ("Forget vault", "zh-CN") => "忘记保险库",
        ("Forget vault", "fr") => "Oublier le coffre",
        ("Forget vault", "ja") => "保管庫を削除",
        ("Forget vault", "hi") => "वॉल्ट भूलें",
        ("Forget vault", "de") => "Tresor vergessen",
        ("Forget vault", "pt-BR") => "Esquecer cofre",
        ("Forget vault", "it") => "Dimentica vault",
        ("Forget vault", "ru") => "Забыть хранилище",
        ("Forget vault", "uk") => "Забути сховище",
        ("Forget vault", "ko") => "보관함 삭제",
        ("Forget vault", "ar") => "نسيان الخزنة",
        ("Forget vault", "tr") => "Kasayı unut",
        ("Forget vault", "pl") => "Usuń sejf",
        ("Forget vault", "nl") => "Kluis vergeten",
        ("Forget vault", "sv") => "Glöm valvet",
        ("Forget vault", "da") => "Glem boks",
        ("Forget vault", "fi") => "Unohda holvi",
        ("Forget vault", "no") => "Glem hvelvet",
        ("Forget vault", "zh-TW") => "忘記保險庫",
        ("Delete vault and secrets", "es") => "Eliminar la bóveda y sus secretos",
        ("Delete vault and secrets", "zh-CN") => "删除保险库和其中的秘密",
        ("Delete vault and secrets", "fr") => "Supprimer le coffre et ses secrets",
        ("Delete vault and secrets", "ja") => "保管庫と秘密を削除",
        ("Delete vault and secrets", "hi") => "वॉल्ट और उसके रहस्य हटाएँ",
        ("Delete vault and secrets", "de") => "Tresor und Geheimnisse löschen",
        ("Delete vault and secrets", "pt-BR") => "Excluir o cofre e seus segredos",
        ("Delete vault and secrets", "it") => "Elimina vault e segreti",
        ("Delete vault and secrets", "ru") => "Удалить хранилище и секреты",
        ("Delete vault and secrets", "uk") => "Видалити сховище та секрети",
        ("Delete vault and secrets", "ko") => "보관함과 비밀 삭제",
        ("Delete vault and secrets", "ar") => "حذف الخزنة والأسرار",
        ("Delete vault and secrets", "tr") => "Kasayı ve sırları sil",
        ("Delete vault and secrets", "pl") => "Usuń sejf i sekrety",
        ("Delete vault and secrets", "nl") => "Kluis en geheimen verwijderen",
        ("Delete vault and secrets", "sv") => "Ta bort valvet och hemligheterna",
        ("Delete vault and secrets", "da") => "Slet boksen og hemmelighederne",
        ("Delete vault and secrets", "fi") => "Poista holvi ja salaisuudet",
        ("Delete vault and secrets", "no") => "Slett hvelvet og hemmelighetene",
        ("Delete vault and secrets", "zh-TW") => "刪除保險庫和其中的秘密",
        (
            "This permanently deletes every saved secret in the active profile.",
            "es",
        ) => "Esto eliminará permanentemente todos los secretos guardados en el perfil activo.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "zh-CN",
        ) => "这将永久删除当前配置中的所有已保存秘密。",
        (
            "This permanently deletes every saved secret in the active profile.",
            "fr",
        ) => "Cela supprimera définitivement tous les secrets enregistrés dans le profil actif.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "ja",
        ) => "現在のプロファイルに保存されているすべての秘密が完全に削除されます。",
        (
            "This permanently deletes every saved secret in the active profile.",
            "hi",
        ) => "इससे सक्रिय प्रोफ़ाइल में सहेजे गए सभी रहस्य स्थायी रूप से हट जाएँगे।",
        (
            "This permanently deletes every saved secret in the active profile.",
            "de",
        ) => "Dadurch werden alle gespeicherten Geheimnisse im aktiven Profil dauerhaft gelöscht.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "pt-BR",
        ) => "Isso excluirá permanentemente todos os segredos salvos no perfil ativo.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "it",
        ) => "Questo eliminerà definitivamente tutti i segreti salvati nel profilo attivo.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "ru",
        ) => "Все сохранённые секреты активного профиля будут удалены без возможности восстановления.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "uk",
        ) => "Усі збережені секрети активного профілю буде видалено без можливості відновлення.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "ko",
        ) => "활성 프로필에 저장된 모든 비밀이 영구적으로 삭제됩니다.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "ar",
        ) => "سيؤدي هذا إلى حذف جميع الأسرار المحفوظة في الملف النشط نهائيًا.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "tr",
        ) => "Bu işlem etkin profildeki tüm kayıtlı sırları kalıcı olarak siler.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "pl",
        ) => "Spowoduje to trwałe usunięcie wszystkich zapisanych sekretów z aktywnego profilu.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "nl",
        ) => "Hiermee worden alle opgeslagen geheimen in het actieve profiel definitief verwijderd.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "sv",
        ) => "Detta tar permanent bort alla sparade hemligheter i den aktiva profilen.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "da",
        ) => "Dette sletter permanent alle gemte hemmeligheder i den aktive profil.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "fi",
        ) => "Tämä poistaa pysyvästi kaikki aktiiviseen profiiliin tallennetut salaisuudet.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "no",
        ) => "Dette sletter alle lagrede hemmeligheter i den aktive profilen permanent.",
        (
            "This permanently deletes every saved secret in the active profile.",
            "zh-TW",
        ) => "這會永久刪除目前設定檔中儲存的所有秘密。",
        (
            "Use a passphrase with at least 12 characters.",
            "es",
        ) => "Usa una frase de contraseña de al menos 12 caracteres.",
        (
            "Use a passphrase with at least 12 characters.",
            "zh-CN",
        ) => "请使用至少 12 个字符的密码短语。",
        (
            "Use a passphrase with at least 12 characters.",
            "fr",
        ) => "Utilisez une phrase secrète d'au moins 12 caractères.",
        (
            "Use a passphrase with at least 12 characters.",
            "ja",
        ) => "12 文字以上のパスフレーズを使用してください。",
        (
            "Use a passphrase with at least 12 characters.",
            "hi",
        ) => "कम से कम 12 अक्षरों वाला पासफ़्रेज़ इस्तेमाल करें।",
        (
            "Use a passphrase with at least 12 characters.",
            "de",
        ) => "Verwende eine Passphrase mit mindestens 12 Zeichen.",
        (
            "Use a passphrase with at least 12 characters.",
            "pt-BR",
        ) => "Use uma frase secreta com pelo menos 12 caracteres.",
        (
            "Use a passphrase with at least 12 characters.",
            "it",
        ) => "Usa una passphrase di almeno 12 caratteri.",
        (
            "Use a passphrase with at least 12 characters.",
            "ru",
        ) => "Используйте парольную фразу длиной не менее 12 символов.",
        (
            "Use a passphrase with at least 12 characters.",
            "uk",
        ) => "Використовуйте парольну фразу щонайменше з 12 символів.",
        (
            "Use a passphrase with at least 12 characters.",
            "ko",
        ) => "12자 이상의 암호 문구를 사용하세요.",
        (
            "Use a passphrase with at least 12 characters.",
            "ar",
        ) => "استخدم عبارة مرور لا تقل عن 12 حرفًا.",
        (
            "Use a passphrase with at least 12 characters.",
            "tr",
        ) => "En az 12 karakterlik bir parola kullanın.",
        (
            "Use a passphrase with at least 12 characters.",
            "pl",
        ) => "Użyj hasła o długości co najmniej 12 znaków.",
        (
            "Use a passphrase with at least 12 characters.",
            "nl",
        ) => "Gebruik een wachtwoordzin van minimaal 12 tekens.",
        (
            "Use a passphrase with at least 12 characters.",
            "sv",
        ) => "Använd en lösenfras med minst 12 tecken.",
        (
            "Use a passphrase with at least 12 characters.",
            "da",
        ) => "Brug en adgangsfrase på mindst 12 tegn.",
        (
            "Use a passphrase with at least 12 characters.",
            "fi",
        ) => "Käytä vähintään 12 merkin tunnuslausetta.",
        (
            "Use a passphrase with at least 12 characters.",
            "no",
        ) => "Bruk en passordfrase på minst 12 tegn.",
        (
            "Use a passphrase with at least 12 characters.",
            "zh-TW",
        ) => "請使用至少 12 個字元的密語。",
        (
            "The passphrase did not unlock this vault.",
            "es",
        ) => "La frase de contraseña no desbloqueó esta bóveda.",
        (
            "The passphrase did not unlock this vault.",
            "zh-CN",
        ) => "此密码短语无法解锁保险库。",
        (
            "The passphrase did not unlock this vault.",
            "fr",
        ) => "Cette phrase secrète n'a pas déverrouillé le coffre.",
        (
            "The passphrase did not unlock this vault.",
            "ja",
        ) => "このパスフレーズでは保管庫を解除できませんでした。",
        (
            "The passphrase did not unlock this vault.",
            "hi",
        ) => "इस पासफ़्रेज़ से वॉल्ट अनलॉक नहीं हुआ।",
        (
            "The passphrase did not unlock this vault.",
            "de",
        ) => "Mit dieser Passphrase ließ sich der Tresor nicht entsperren.",
        (
            "The passphrase did not unlock this vault.",
            "pt-BR",
        ) => "Esta frase secreta não desbloqueou o cofre.",
        (
            "The passphrase did not unlock this vault.",
            "it",
        ) => "Questa passphrase non ha sbloccato il vault.",
        (
            "The passphrase did not unlock this vault.",
            "ru",
        ) => "Эта парольная фраза не открыла хранилище.",
        (
            "The passphrase did not unlock this vault.",
            "uk",
        ) => "Ця парольна фраза не розблокувала сховище.",
        (
            "The passphrase did not unlock this vault.",
            "ko",
        ) => "이 암호 문구로 보관함을 잠금 해제하지 못했습니다.",
        (
            "The passphrase did not unlock this vault.",
            "ar",
        ) => "لم تفتح عبارة المرور هذه الخزنة.",
        (
            "The passphrase did not unlock this vault.",
            "tr",
        ) => "Bu parola kasanın kilidini açmadı.",
        (
            "The passphrase did not unlock this vault.",
            "pl",
        ) => "To hasło nie odblokowało sejfu.",
        (
            "The passphrase did not unlock this vault.",
            "nl",
        ) => "Met deze wachtwoordzin is de kluis niet ontgrendeld.",
        (
            "The passphrase did not unlock this vault.",
            "sv",
        ) => "Lösenfrasen låste inte upp valvet.",
        (
            "The passphrase did not unlock this vault.",
            "da",
        ) => "Adgangsfrasen låste ikke boksen op.",
        (
            "The passphrase did not unlock this vault.",
            "fi",
        ) => "Tunnuslause ei avannut holvia.",
        (
            "The passphrase did not unlock this vault.",
            "no",
        ) => "Passordfrasen låste ikke opp hvelvet.",
        (
            "The passphrase did not unlock this vault.",
            "zh-TW",
        ) => "此密語無法解鎖保險庫。",
        (
            "Quick Secrets could not complete this action.",
            "es",
        ) => "Quick Secrets no pudo completar esta acción.",
        (
            "Quick Secrets could not complete this action.",
            "zh-CN",
        ) => "Quick Secrets 无法完成此操作。",
        (
            "Quick Secrets could not complete this action.",
            "fr",
        ) => "Quick Secrets n'a pas pu terminer cette action.",
        (
            "Quick Secrets could not complete this action.",
            "ja",
        ) => "Quick Secrets はこの操作を完了できませんでした。",
        (
            "Quick Secrets could not complete this action.",
            "hi",
        ) => "Quick Secrets यह कार्रवाई पूरी नहीं कर सका।",
        (
            "Quick Secrets could not complete this action.",
            "de",
        ) => "Quick Secrets konnte diese Aktion nicht abschließen.",
        (
            "Quick Secrets could not complete this action.",
            "pt-BR",
        ) => "O Quick Secrets não conseguiu concluir esta ação.",
        (
            "Quick Secrets could not complete this action.",
            "it",
        ) => "Quick Secrets non è riuscito a completare questa azione.",
        (
            "Quick Secrets could not complete this action.",
            "ru",
        ) => "Quick Secrets не удалось выполнить это действие.",
        (
            "Quick Secrets could not complete this action.",
            "uk",
        ) => "Quick Secrets не вдалося виконати цю дію.",
        (
            "Quick Secrets could not complete this action.",
            "ko",
        ) => "Quick Secrets에서 이 작업을 완료하지 못했습니다.",
        (
            "Quick Secrets could not complete this action.",
            "ar",
        ) => "تعذر على Quick Secrets إكمال هذا الإجراء.",
        (
            "Quick Secrets could not complete this action.",
            "tr",
        ) => "Quick Secrets bu işlemi tamamlayamadı.",
        (
            "Quick Secrets could not complete this action.",
            "pl",
        ) => "Quick Secrets nie udało się ukończyć tej czynności.",
        (
            "Quick Secrets could not complete this action.",
            "nl",
        ) => "Quick Secrets kon deze actie niet voltooien.",
        (
            "Quick Secrets could not complete this action.",
            "sv",
        ) => "Quick Secrets kunde inte slutföra åtgärden.",
        (
            "Quick Secrets could not complete this action.",
            "da",
        ) => "Quick Secrets kunne ikke fuldføre handlingen.",
        (
            "Quick Secrets could not complete this action.",
            "fi",
        ) => "Quick Secrets ei voinut suorittaa tätä toimintoa.",
        (
            "Quick Secrets could not complete this action.",
            "no",
        ) => "Quick Secrets kunne ikke fullføre denne handlingen.",
        (
            "Quick Secrets could not complete this action.",
            "zh-TW",
        ) => "Quick Secrets 無法完成此操作。",
        _ => return None,
    };
    Some(value)
}

fn catalog_alias(english: &str) -> &str {
    match english {
        "Commands" => "Command",
        "Theme Library" => "Themes",
        "Apply:" => "Apply",
        "Shell Profiles" => "Custom Shell Profiles",
        "Open a terminal" | "New terminal with…" => "New terminal",
        "Run" | "Run in terminal" => "Run preset",
        "Add a preset" => "Add new preset",
        "Install AI coding tools" => "Open vibe install guide",
        "+ Add SSH preset" => "Add Preset",
        "Delete" => "Delete preset",
        "Tab title" => "Tab title (optional)",
        _ => english,
    }
}

pub fn formatted_literal(locale: &str, english: &str, args: &[(&str, &str)]) -> String {
    interpolate(literal(locale, english), args)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageKey {
    ImportFromOriginal,
    ImportPreview,
    ImportConfirm,
    ImportExcludedKeys,
    ImportConflict,
    AutomationControl,
    AutomationLocked,
    AgentInst,
    AgentInstructionsCopied,
    AiHelp,
    AiHelpExplain,
    AiHelpSuggest,
    AiHelpReview,
    AiHelpRun,
    Cancel,
    ChromeCornerRadius,
    ChromeCornerRadiusHelp,
    CurrentTerminalTheme,
    RandomCurrent,
    RandomAll,
    ThisTerminal,
    ThemeAll,
    ThemeAllHelp,
    UseGlobalTheme,
    TabThemeTooltip,
    Providers,
    AddProvider,
    RemoveProvider,
    ProviderName,
    ProviderEndpoint,
    ProviderModel,
    ActiveProvider,
    ApiKey,
    SaveKey,
    RemoveKey,
    SessionOnlyKey,
    ProviderKeyStatus,
    ProviderEndpointInvalid,
    ProviderHelp,
    ImportKeyCount,
    ImportKeysChoice,
    ImportOtherExclusions,
    ProviderSessionStatus,
    ProviderOsStatus,
    ProviderNoneStatus,
    KeySavedSession,
    KeySavedOs,
    KeySaveFailed,
    KeyRemoved,
    KeyRemoveFailed,
    ImportKeysResult,
    ImportKeysFailed,
    TestConnection,
    DiscoverModels,
    ConnectionSucceeded,
    ModelsFound,
    AiHelpLockedProvider,
    AiHelpDescription,
    AiHelpProviderSettings,
    AiHelpConversationSession,
    AiHelpReviewCommand,
    AiHelpTarget,
    AiHelpWouldEnter,
    AiHelpWouldNotEnter,
    AiHelpInsert,
    AiHelpInsertEnter,
    AiHelpReviewControl,
    AiHelpTerminalKey,
    AiHelpSendReviewedKey,
    AiHelpNoTarget,
    AiHelpWaitingProvider,
    AiHelpIncludeContext,
    AiHelpPreviewContext,
    AiHelpPreparingContext,
    AiHelpIncludedTerminal,
    AiHelpContextSentPrivacy,
    AiHelpQuestionHint,
    AiHelpSend,
    AiHelpRetry,
    AiHelpRequiresPro,
    AiHelpProviderMissing,
    AiHelpEndpointMissing,
    AiHelpContextTerminalMissing,
    AiHelpCredentialRedactionFailed,
    AiHelpRequestCancelled,
    AiHelpInputSent,
    AiHelpDeliveryFailed,
    AiHelpRequestFailed,
    AiHelpYou,
    AiHelpQuestionTooLong,
    Shortcuts,
    ShortcutHelp,
    ShortcutRecord,
    ShortcutRecordPrompt,
    ShortcutConflict,
    ShortcutUnsafeInterrupt,
    ShortcutModifierRequired,
    ShortcutClear,
    ShortcutResetDefaults,
    ShortcutSaved,
    ShortcutCleared,
    ShortcutResetComplete,
    ShortcutInvalid,
    ShortcutConflictUnknown,
    ShortcutNewTab,
    ShortcutCloseTab,
    ShortcutReopenTab,
    ShortcutCopy,
    ShortcutPaste,
    ShortcutOpenSettings,
    ShortcutQuit,
    TerminalFind,
    TerminalSearchHint,
    TerminalSearchNext,
    TerminalSearchPrevious,
    TerminalSearchClear,
    TerminalSelectAll,
    TerminalClearScreen,
    TerminalSearchStatus,
    TerminalSearchNoMatches,
    TerminalSearchInvalidPattern,
    WorkspaceDockWidth,
    WorkspaceDockCompact,
    WorkspaceDockAutoHide,
    WorkspaceDockAutoHideDelay,
    WorkspaceDockOpacity,
    WorkspaceDockPeekRadius,
    WorkspaceDockShow,
    WindowOpacity,
    WindowOpacitySupported,
    WindowOpacityUnsupported,
    WindowOpacityFailed,
    TerminalZoomIn,
    TerminalZoomOut,
    TerminalZoomReset,
    CalmMode,
    CalmModeHelp,
    SettingsTitle,
    SettingsKeepClose,
    SettingsRevertClose,
    LegacyShaderNotRendered,
}

impl MessageKey {
    pub const ALL: [Self; 141] = [
        Self::ImportFromOriginal,
        Self::ImportPreview,
        Self::ImportConfirm,
        Self::ImportExcludedKeys,
        Self::ImportConflict,
        Self::AutomationControl,
        Self::AutomationLocked,
        Self::AgentInst,
        Self::AgentInstructionsCopied,
        Self::AiHelp,
        Self::AiHelpExplain,
        Self::AiHelpSuggest,
        Self::AiHelpReview,
        Self::AiHelpRun,
        Self::Cancel,
        Self::ChromeCornerRadius,
        Self::ChromeCornerRadiusHelp,
        Self::CurrentTerminalTheme,
        Self::RandomCurrent,
        Self::RandomAll,
        Self::ThisTerminal,
        Self::ThemeAll,
        Self::ThemeAllHelp,
        Self::UseGlobalTheme,
        Self::TabThemeTooltip,
        Self::Providers,
        Self::AddProvider,
        Self::RemoveProvider,
        Self::ProviderName,
        Self::ProviderEndpoint,
        Self::ProviderModel,
        Self::ActiveProvider,
        Self::ApiKey,
        Self::SaveKey,
        Self::RemoveKey,
        Self::SessionOnlyKey,
        Self::ProviderKeyStatus,
        Self::ProviderEndpointInvalid,
        Self::ProviderHelp,
        Self::ImportKeyCount,
        Self::ImportKeysChoice,
        Self::ImportOtherExclusions,
        Self::ProviderSessionStatus,
        Self::ProviderOsStatus,
        Self::ProviderNoneStatus,
        Self::KeySavedSession,
        Self::KeySavedOs,
        Self::KeySaveFailed,
        Self::KeyRemoved,
        Self::KeyRemoveFailed,
        Self::ImportKeysResult,
        Self::ImportKeysFailed,
        Self::TestConnection,
        Self::DiscoverModels,
        Self::ConnectionSucceeded,
        Self::ModelsFound,
        Self::AiHelpLockedProvider,
        Self::AiHelpDescription,
        Self::AiHelpProviderSettings,
        Self::AiHelpConversationSession,
        Self::AiHelpReviewCommand,
        Self::AiHelpTarget,
        Self::AiHelpWouldEnter,
        Self::AiHelpWouldNotEnter,
        Self::AiHelpInsert,
        Self::AiHelpInsertEnter,
        Self::AiHelpReviewControl,
        Self::AiHelpTerminalKey,
        Self::AiHelpSendReviewedKey,
        Self::AiHelpNoTarget,
        Self::AiHelpWaitingProvider,
        Self::AiHelpIncludeContext,
        Self::AiHelpPreviewContext,
        Self::AiHelpPreparingContext,
        Self::AiHelpIncludedTerminal,
        Self::AiHelpContextSentPrivacy,
        Self::AiHelpQuestionHint,
        Self::AiHelpSend,
        Self::AiHelpRetry,
        Self::AiHelpRequiresPro,
        Self::AiHelpProviderMissing,
        Self::AiHelpEndpointMissing,
        Self::AiHelpContextTerminalMissing,
        Self::AiHelpCredentialRedactionFailed,
        Self::AiHelpRequestCancelled,
        Self::AiHelpInputSent,
        Self::AiHelpDeliveryFailed,
        Self::AiHelpRequestFailed,
        Self::AiHelpYou,
        Self::AiHelpQuestionTooLong,
        Self::Shortcuts,
        Self::ShortcutHelp,
        Self::ShortcutRecord,
        Self::ShortcutRecordPrompt,
        Self::ShortcutConflict,
        Self::ShortcutUnsafeInterrupt,
        Self::ShortcutModifierRequired,
        Self::ShortcutClear,
        Self::ShortcutResetDefaults,
        Self::ShortcutSaved,
        Self::ShortcutCleared,
        Self::ShortcutResetComplete,
        Self::ShortcutInvalid,
        Self::ShortcutConflictUnknown,
        Self::ShortcutNewTab,
        Self::ShortcutCloseTab,
        Self::ShortcutReopenTab,
        Self::ShortcutCopy,
        Self::ShortcutPaste,
        Self::ShortcutOpenSettings,
        Self::ShortcutQuit,
        Self::TerminalFind,
        Self::TerminalSearchHint,
        Self::TerminalSearchNext,
        Self::TerminalSearchPrevious,
        Self::TerminalSearchClear,
        Self::TerminalSelectAll,
        Self::TerminalClearScreen,
        Self::TerminalSearchStatus,
        Self::TerminalSearchNoMatches,
        Self::TerminalSearchInvalidPattern,
        Self::WorkspaceDockWidth,
        Self::WorkspaceDockCompact,
        Self::WorkspaceDockAutoHide,
        Self::WorkspaceDockAutoHideDelay,
        Self::WorkspaceDockOpacity,
        Self::WorkspaceDockPeekRadius,
        Self::WorkspaceDockShow,
        Self::WindowOpacity,
        Self::WindowOpacitySupported,
        Self::WindowOpacityUnsupported,
        Self::WindowOpacityFailed,
        Self::TerminalZoomIn,
        Self::TerminalZoomOut,
        Self::TerminalZoomReset,
        Self::CalmMode,
        Self::CalmModeHelp,
        Self::SettingsTitle,
        Self::SettingsKeepClose,
        Self::SettingsRevertClose,
        Self::LegacyShaderNotRendered,
    ];
}

pub fn resolve_locale(requested: &str) -> &'static str {
    let normalized = requested.trim().replace('_', "-");
    if let Some(locale) = SUPPORTED_LOCALES
        .iter()
        .find(|code| code.eq_ignore_ascii_case(&normalized))
    {
        return locale;
    }
    let language = normalized.split('-').next().unwrap_or("");
    SUPPORTED_LOCALES
        .iter()
        .copied()
        .find(|code| code.eq_ignore_ascii_case(language))
        .unwrap_or("en")
}

pub fn locale_info(requested: &str) -> &'static LocaleInfo {
    let locale = resolve_locale(requested);
    LOCALE_INFO
        .iter()
        .find(|info| info.code == locale)
        .unwrap_or(&LOCALE_INFO[0])
}

pub fn system_locale() -> &'static str {
    #[cfg(target_os = "windows")]
    let detected = windows_system_locale();
    #[cfg(not(target_os = "windows"))]
    let detected = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|key| std::env::var(key).ok().filter(|value| !value.is_empty()));

    detected
        .as_deref()
        .map(|value| value.split(['.', '@']).next().unwrap_or(value))
        .map(resolve_locale)
        .unwrap_or("en")
}

#[cfg(target_os = "windows")]
fn windows_system_locale() -> Option<String> {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buffer = [0_u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    (length > 1).then(|| String::from_utf16_lossy(&buffer[..length as usize - 1]))
}

fn english(key: MessageKey) -> &'static str {
    match key {
        MessageKey::ImportFromOriginal => "Import from original ButtonsCLI",
        MessageKey::ImportPreview => "Preview import",
        MessageKey::ImportConfirm => "Import into {profile}",
        MessageKey::ImportExcludedKeys => "API keys are excluded unless selected separately.",
        MessageKey::ImportConflict => "{count} item(s) need a conflict choice.",
        MessageKey::AutomationControl => "Agent control",
        MessageKey::AutomationLocked => "Agent control requires Pro access.",
        MessageKey::AgentInst => "Agent Inst.",
        MessageKey::AgentInstructionsCopied => "Native agent instructions copied to clipboard.",
        MessageKey::AiHelp => "AI Help",
        MessageKey::AiHelpExplain => "Explain this terminal",
        MessageKey::AiHelpSuggest => "Suggest a command",
        MessageKey::AiHelpReview => "Review command before sending",
        MessageKey::AiHelpRun => "Run reviewed command",
        MessageKey::Cancel => "Cancel",
        MessageKey::ChromeCornerRadius => "Chrome corner radius",
        MessageKey::ChromeCornerRadiusHelp => {
            "Rounds tabs, controls, cards, menus, and Settings without changing terminal cells."
        }
        MessageKey::CurrentTerminalTheme => "Current terminal theme",
        MessageKey::RandomCurrent => "Random current",
        MessageKey::RandomAll => "Random all",
        MessageKey::ThisTerminal => "This terminal",
        MessageKey::ThemeAll => "Theme all",
        MessageKey::ThemeAllHelp => "Set this terminal theme as the default for new tabs and replace overrides in every open tab.",
        MessageKey::UseGlobalTheme => "Use global",
        MessageKey::TabThemeTooltip => "Double-click to rename · Theme: {name}",
        MessageKey::Providers => "AI providers",
        MessageKey::AddProvider => "Add provider",
        MessageKey::RemoveProvider => "Remove provider",
        MessageKey::ProviderName => "Name",
        MessageKey::ProviderEndpoint => "Chat completions endpoint",
        MessageKey::ProviderModel => "Model ID",
        MessageKey::ActiveProvider => "Active provider",
        MessageKey::ApiKey => "API key",
        MessageKey::SaveKey => "Save key",
        MessageKey::RemoveKey => "Remove saved key",
        MessageKey::SessionOnlyKey => "Keep key for this session only",
        MessageKey::ProviderKeyStatus => "Saved key: {status}",
        MessageKey::ProviderEndpointInvalid => "The endpoint must be an HTTP or HTTPS URL without credentials, query, or fragment.",
        MessageKey::ProviderHelp => "Set a chat completions endpoint and model. Keys are kept in your operating system credential store or only in memory for this session.",
        MessageKey::ImportKeyCount => "{count} API key(s) found in the original profile.",
        MessageKey::ImportKeysChoice => "Also transfer keys for imported providers to this operating system's credential store",
        MessageKey::ImportOtherExclusions => "Runtime/auth files, session history and unknown top-level fields are excluded.",
        MessageKey::ProviderSessionStatus => "session",
        MessageKey::ProviderOsStatus => "operating system",
        MessageKey::ProviderNoneStatus => "none",
        MessageKey::KeySavedSession => "Key is available until this app closes.",
        MessageKey::KeySavedOs => "Key saved to the operating system credential store.",
        MessageKey::KeySaveFailed => "Key was not saved: {reason}",
        MessageKey::KeyRemoved => "Key removed.",
        MessageKey::KeyRemoveFailed => "Key was not removed: {reason}",
        MessageKey::ImportKeysResult => "Import complete. {saved} API key(s) saved; {failed} could not be saved. New terminals use the imported profile.",
        MessageKey::ImportKeysFailed => "Settings imported, but API keys could not be transferred: {reason}",
        MessageKey::TestConnection => "Test connection",
        MessageKey::DiscoverModels => "Discover models",
        MessageKey::ConnectionSucceeded => "Provider returned a valid chat response.",
        MessageKey::ModelsFound => "Found {count} models.",
        MessageKey::AiHelpLockedProvider => "AI Help is a Pro feature. In development builds, set BUTTONSCLI_NATIVE_DEV_AI_HELP=1 to exercise the provider connection tools.",
        MessageKey::AiHelpDescription => "Ask a question or request a command. Suggested terminal actions require separate review.",
        MessageKey::AiHelpProviderSettings => "Open provider settings",
        MessageKey::AiHelpConversationSession => "Your conversation stays in this window for this session.",
        MessageKey::AiHelpReviewCommand => "Review command: {label}",
        MessageKey::AiHelpTarget => "Target: {target}",
        MessageKey::AiHelpWouldEnter => "Would press Enter after insertion.",
        MessageKey::AiHelpWouldNotEnter => "Would insert without pressing Enter.",
        MessageKey::AiHelpInsert => "Insert",
        MessageKey::AiHelpInsertEnter => "Insert + Enter",
        MessageKey::AiHelpReviewControl => "Review control: {label}",
        MessageKey::AiHelpTerminalKey => "Terminal key: {key}",
        MessageKey::AiHelpSendReviewedKey => "Send reviewed key",
        MessageKey::AiHelpNoTarget => "No terminal target",
        MessageKey::AiHelpWaitingProvider => "Waiting for provider…",
        MessageKey::AiHelpIncludeContext => "Include a terminal output snapshot",
        MessageKey::AiHelpPreviewContext => "Preview active terminal context",
        MessageKey::AiHelpPreparingContext => "Preparing a redacted snapshot…",
        MessageKey::AiHelpIncludedTerminal => "Included terminal: {title} · {shell} · session {id}",
        MessageKey::AiHelpContextSentPrivacy => "The title, shell, and previewed text above are sent with your question. Secret redaction is best effort.",
        MessageKey::AiHelpQuestionHint => "Ask AI Help…",
        MessageKey::AiHelpSend => "Send",
        MessageKey::AiHelpRetry => "Retry last request",
        MessageKey::AiHelpRequiresPro => "AI Help requires Pro access.",
        MessageKey::AiHelpProviderMissing => "Add an AI provider in Settings first.",
        MessageKey::AiHelpEndpointMissing => "Set a valid endpoint and model in provider settings.",
        MessageKey::AiHelpContextTerminalMissing => "Open a terminal before previewing its context.",
        MessageKey::AiHelpCredentialRedactionFailed => "Could not read the configured API key for redaction: {reason}",
        MessageKey::AiHelpRequestCancelled => "Request cancelled.",
        MessageKey::AiHelpInputSent => "Reviewed input sent to terminal {id}.",
        MessageKey::AiHelpDeliveryFailed => "Terminal action failed: {reason}",
        MessageKey::AiHelpRequestFailed => "Provider request failed: {reason}",
        MessageKey::AiHelpYou => "You",
        MessageKey::AiHelpQuestionTooLong => "Keep the question under 16,384 characters.",
        MessageKey::Shortcuts => "Shortcuts",
        MessageKey::ShortcutHelp => "Set app shortcuts. Primary means Ctrl on Windows/Linux and Command on macOS. Ctrl+C stays available to interrupt the shell.",
        MessageKey::ShortcutRecord => "Record",
        MessageKey::ShortcutRecordPrompt => "Press the key combination to assign. Escape cancels.",
        MessageKey::ShortcutConflict => "That shortcut is already assigned to {action}.",
        MessageKey::ShortcutUnsafeInterrupt => "Ctrl+C is reserved for terminal interrupt. Primary+C without Shift is blocked to keep the setting safe on Windows and Linux.",
        MessageKey::ShortcutModifierRequired => "Include Primary, Ctrl, or Alt so ordinary terminal typing is not intercepted.",
        MessageKey::ShortcutClear => "Clear",
        MessageKey::ShortcutResetDefaults => "Restore defaults",
        MessageKey::ShortcutSaved => "Shortcut saved.",
        MessageKey::ShortcutCleared => "Shortcut cleared.",
        MessageKey::ShortcutResetComplete => "Default shortcuts restored.",
        MessageKey::ShortcutInvalid => "That key or combination is not supported.",
        MessageKey::ShortcutConflictUnknown => "That shortcut is reserved by a saved action from another app version.",
        MessageKey::ShortcutNewTab => "New tab",
        MessageKey::ShortcutCloseTab => "Close tab",
        MessageKey::ShortcutReopenTab => "Reopen tab",
        MessageKey::ShortcutCopy => "Copy selection",
        MessageKey::ShortcutPaste => "Paste",
        MessageKey::ShortcutOpenSettings => "Open Settings",
        MessageKey::ShortcutQuit => "Quit",
        MessageKey::TerminalFind => "Find in terminal…",
        MessageKey::TerminalSearchHint => "Search text or regex",
        MessageKey::TerminalSearchNext => "Next",
        MessageKey::TerminalSearchPrevious => "Previous",
        MessageKey::TerminalSearchClear => "Clear search",
        MessageKey::TerminalSelectAll => "Select all",
        MessageKey::TerminalClearScreen => "Clear screen",
        MessageKey::TerminalSearchStatus => "{current} of {count}",
        MessageKey::TerminalSearchNoMatches => "No matches",
        MessageKey::TerminalSearchInvalidPattern => "Invalid search pattern",
        MessageKey::WorkspaceDockWidth => "Command dock width",
        MessageKey::WorkspaceDockCompact => "Compact command dock",
        MessageKey::WorkspaceDockAutoHide => "Auto-hide command dock",
        MessageKey::WorkspaceDockAutoHideDelay => "Auto-hide delay",
        MessageKey::WorkspaceDockOpacity => "Auto-hide rail opacity",
        MessageKey::WorkspaceDockPeekRadius => "Peek area",
        MessageKey::WorkspaceDockShow => "Show command dock",
        MessageKey::WindowOpacity => "Adjust main window opacity",
        MessageKey::WindowOpacitySupported => "Window opacity is supported on Windows.",
        MessageKey::WindowOpacityUnsupported => "Window opacity is not supported on this platform.",
        MessageKey::WindowOpacityFailed => "Window opacity could not be changed: {error}",
        MessageKey::TerminalZoomIn => "Increase terminal text size",
        MessageKey::TerminalZoomOut => "Decrease terminal text size",
        MessageKey::TerminalZoomReset => "Reset terminal text size",
        MessageKey::CalmMode => "Calm effects",
        MessageKey::CalmModeHelp => "Pause animated terminal effects",
        MessageKey::SettingsTitle => "ButtonsCLI Settings",
        MessageKey::SettingsKeepClose => "Keep changes and close",
        MessageKey::SettingsRevertClose => "Revert and close",
        MessageKey::LegacyShaderNotRendered => {
            "This theme’s legacy GLSL shader is preserved but not run by native rendering."
        }
    }
}

fn override_text(locale: &str, key: MessageKey) -> Option<&'static str> {
    // Like the original catalog, partial locale entries fall back to English.
    match (locale, key) {
        ("en", MessageKey::LegacyShaderNotRendered) => Some("This theme’s legacy GLSL shader is preserved but not run by native rendering."),
        ("es", MessageKey::LegacyShaderNotRendered) => Some("El shader GLSL heredado de este tema se conserva, pero no se ejecuta en el renderizado nativo."),
        ("zh-CN", MessageKey::LegacyShaderNotRendered) => Some("此主题的旧版 GLSL 着色器会保留，但不会在原生渲染中运行。"),
        ("fr", MessageKey::LegacyShaderNotRendered) => Some("Le shader GLSL historique de ce thème est conservé, mais n’est pas exécuté dans le rendu natif."),
        ("ja", MessageKey::LegacyShaderNotRendered) => Some("このテーマの旧式 GLSL シェーダーは保持されますが、ネイティブ描画では実行されません。"),
        ("hi", MessageKey::LegacyShaderNotRendered) => Some("इस थीम का पुराना GLSL शेडर सुरक्षित रखा गया है, लेकिन नेटिव रेंडरिंग में नहीं चलता।"),
        ("de", MessageKey::LegacyShaderNotRendered) => Some("Der ältere GLSL-Shader dieses Themes bleibt erhalten, wird nativ aber nicht ausgeführt."),
        ("pt-BR", MessageKey::LegacyShaderNotRendered) => Some("O shader GLSL legado deste tema é preservado, mas não é executado na renderização nativa."),
        ("it", MessageKey::LegacyShaderNotRendered) => Some("Lo shader GLSL legacy di questo tema viene conservato, ma non viene eseguito nel rendering nativo."),
        ("ru", MessageKey::LegacyShaderNotRendered) => Some("Устаревший шейдер GLSL этой темы сохранён, но в нативной отрисовке не выполняется."),
        ("uk", MessageKey::LegacyShaderNotRendered) => Some("Застарілий шейдер GLSL цієї теми збережено, але він не виконується в нативному рендерингу."),
        ("ko", MessageKey::LegacyShaderNotRendered) => Some("이 테마의 레거시 GLSL 셰이더는 보존되지만 기본 렌더링에서는 실행되지 않습니다."),
        ("ar", MessageKey::LegacyShaderNotRendered) => Some("يُحتفَظ بمظلّل GLSL القديم لهذا المظهر، لكنه لا يعمل في العرض الأصلي."),
        ("tr", MessageKey::LegacyShaderNotRendered) => Some("Bu temanın eski GLSL gölgelendiricisi korunur, ancak yerel çizimde çalıştırılmaz."),
        ("pl", MessageKey::LegacyShaderNotRendered) => Some("Starszy shader GLSL tego motywu zostaje zachowany, ale nie jest uruchamiany w renderowaniu natywnym."),
        ("nl", MessageKey::LegacyShaderNotRendered) => Some("De oudere GLSL-shader van dit thema blijft behouden, maar wordt niet uitgevoerd in de native weergave."),
        ("sv", MessageKey::LegacyShaderNotRendered) => Some("Temats äldre GLSL-shader bevaras, men körs inte i den inbyggda renderingen."),
        ("da", MessageKey::LegacyShaderNotRendered) => Some("Temaets ældre GLSL-shader bevares, men køres ikke i den indbyggede rendering."),
        ("fi", MessageKey::LegacyShaderNotRendered) => Some("Teeman vanha GLSL-varjostin säilyy, mutta sitä ei suoriteta natiivissa renderöinnissä."),
        ("no", MessageKey::LegacyShaderNotRendered) => Some("Temaets eldre GLSL-shader bevares, men kjøres ikke i den innebygde gjengivelsen."),
        ("zh-TW", MessageKey::LegacyShaderNotRendered) => Some("此主題的舊版 GLSL 著色器會保留，但不會在原生繪製中執行。"),
        ("es", MessageKey::ImportFromOriginal) => Some("Importar desde ButtonsCLI original"),
        ("es", MessageKey::AiHelp) => Some("Ayuda de IA"),
        ("es", MessageKey::Cancel) => Some("Cancelar"),
        ("fr", MessageKey::ImportFromOriginal) => Some("Importer depuis ButtonsCLI original"),
        ("fr", MessageKey::AiHelp) => Some("Aide IA"),
        ("fr", MessageKey::Cancel) => Some("Annuler"),
        ("ja", MessageKey::AiHelp) => Some("AI ヘルプ"),
        ("de", MessageKey::AiHelp) => Some("KI-Hilfe"),
        ("es", MessageKey::WindowOpacitySupported) => {
            Some("La opacidad de la ventana es compatible con Windows.")
        }
        ("es", MessageKey::WindowOpacityUnsupported) => {
            Some("La opacidad de la ventana no es compatible con esta plataforma.")
        }
        ("es", MessageKey::WindowOpacityFailed) => {
            Some("No se pudo cambiar la opacidad de la ventana: {error}")
        }
        ("zh-CN", MessageKey::WindowOpacitySupported) => Some("Windows 支持窗口透明度。"),
        ("zh-CN", MessageKey::WindowOpacityUnsupported) => Some("此平台不支持窗口透明度。"),
        ("zh-CN", MessageKey::WindowOpacityFailed) => Some("无法更改窗口透明度：{error}"),
        ("fr", MessageKey::WindowOpacitySupported) => {
            Some("L’opacité de la fenêtre est prise en charge sous Windows.")
        }
        ("fr", MessageKey::WindowOpacityUnsupported) => {
            Some("L’opacité de la fenêtre n’est pas prise en charge sur cette plateforme.")
        }
        ("fr", MessageKey::WindowOpacityFailed) => {
            Some("Impossible de modifier l’opacité de la fenêtre : {error}")
        }
        ("hi", MessageKey::WindowOpacitySupported) => {
            Some("Windows पर विंडो की पारदर्शिता समर्थित है।")
        }
        ("hi", MessageKey::WindowOpacityUnsupported) => {
            Some("इस प्लेटफ़ॉर्म पर विंडो पारदर्शिता समर्थित नहीं है।")
        }
        ("hi", MessageKey::WindowOpacityFailed) => Some("विंडो पारदर्शिता बदली नहीं जा सकी: {error}"),
        ("ja", MessageKey::WindowOpacitySupported) => {
            Some("ウィンドウの不透明度は Windows で利用できます。")
        }
        ("ja", MessageKey::WindowOpacityUnsupported) => {
            Some("このプラットフォームではウィンドウの不透明度を変更できません。")
        }
        ("ja", MessageKey::WindowOpacityFailed) => {
            Some("ウィンドウの不透明度を変更できませんでした: {error}")
        }
        ("de", MessageKey::WindowOpacitySupported) => {
            Some("Fenstertransparenz wird unter Windows unterstützt.")
        }
        ("de", MessageKey::WindowOpacityUnsupported) => {
            Some("Fenstertransparenz wird auf dieser Plattform nicht unterstützt.")
        }
        ("de", MessageKey::WindowOpacityFailed) => {
            Some("Fenstertransparenz konnte nicht geändert werden: {error}")
        }
        ("pt-BR", MessageKey::WindowOpacitySupported) => {
            Some("A opacidade da janela é compatível com o Windows.")
        }
        ("pt-BR", MessageKey::WindowOpacityUnsupported) => {
            Some("A opacidade da janela não é compatível com esta plataforma.")
        }
        ("pt-BR", MessageKey::WindowOpacityFailed) => {
            Some("Não foi possível alterar a opacidade da janela: {error}")
        }
        ("it", MessageKey::WindowOpacitySupported) => {
            Some("L’opacità della finestra è supportata su Windows.")
        }
        ("it", MessageKey::WindowOpacityUnsupported) => {
            Some("L’opacità della finestra non è supportata su questa piattaforma.")
        }
        ("it", MessageKey::WindowOpacityFailed) => {
            Some("Impossibile modificare l’opacità della finestra: {error}")
        }
        ("ru", MessageKey::WindowOpacitySupported) => {
            Some("Настройка прозрачности окна поддерживается в Windows.")
        }
        ("ru", MessageKey::WindowOpacityUnsupported) => {
            Some("Настройка прозрачности окна не поддерживается на этой платформе.")
        }
        ("ru", MessageKey::WindowOpacityFailed) => {
            Some("Не удалось изменить прозрачность окна: {error}")
        }
        ("uk", MessageKey::WindowOpacitySupported) => {
            Some("Налаштування прозорості вікна підтримується у Windows.")
        }
        ("uk", MessageKey::WindowOpacityUnsupported) => {
            Some("Налаштування прозорості вікна не підтримується на цій платформі.")
        }
        ("uk", MessageKey::WindowOpacityFailed) => {
            Some("Не вдалося змінити прозорість вікна: {error}")
        }
        ("ko", MessageKey::WindowOpacitySupported) => Some("Windows에서는 창 투명도를 지원합니다."),
        ("ko", MessageKey::WindowOpacityUnsupported) => {
            Some("이 플랫폼에서는 창 투명도를 지원하지 않습니다.")
        }
        ("ko", MessageKey::WindowOpacityFailed) => Some("창 투명도를 변경하지 못했습니다: {error}"),
        ("ar", MessageKey::WindowOpacitySupported) => Some("شفافية النافذة مدعومة على Windows."),
        ("ar", MessageKey::WindowOpacityUnsupported) => {
            Some("شفافية النافذة غير مدعومة على هذه المنصة.")
        }
        ("ar", MessageKey::WindowOpacityFailed) => Some("تعذر تغيير شفافية النافذة: {error}"),
        ("tr", MessageKey::WindowOpacitySupported) => {
            Some("Pencere saydamlığı Windows'ta desteklenir.")
        }
        ("tr", MessageKey::WindowOpacityUnsupported) => {
            Some("Pencere saydamlığı bu platformda desteklenmiyor.")
        }
        ("tr", MessageKey::WindowOpacityFailed) => {
            Some("Pencere saydamlığı değiştirilemedi: {error}")
        }
        ("pl", MessageKey::WindowOpacitySupported) => {
            Some("Przezroczystość okna jest obsługiwana w systemie Windows.")
        }
        ("pl", MessageKey::WindowOpacityUnsupported) => {
            Some("Przezroczystość okna nie jest obsługiwana na tej platformie.")
        }
        ("pl", MessageKey::WindowOpacityFailed) => {
            Some("Nie udało się zmienić przezroczystości okna: {error}")
        }
        ("nl", MessageKey::WindowOpacitySupported) => {
            Some("Vensterdoorzichtigheid wordt ondersteund in Windows.")
        }
        ("nl", MessageKey::WindowOpacityUnsupported) => {
            Some("Vensterdoorzichtigheid wordt niet ondersteund op dit platform.")
        }
        ("nl", MessageKey::WindowOpacityFailed) => {
            Some("De vensterdoorzichtigheid kon niet worden gewijzigd: {error}")
        }
        ("sv", MessageKey::WindowOpacitySupported) => Some("Fönsteropacitet stöds i Windows."),
        ("sv", MessageKey::WindowOpacityUnsupported) => {
            Some("Fönsteropacitet stöds inte på den här plattformen.")
        }
        ("sv", MessageKey::WindowOpacityFailed) => {
            Some("Det gick inte att ändra fönsteropaciteten: {error}")
        }
        ("da", MessageKey::WindowOpacitySupported) => {
            Some("Vinduesgennemsigtighed understøttes i Windows.")
        }
        ("da", MessageKey::WindowOpacityUnsupported) => {
            Some("Vinduesgennemsigtighed understøttes ikke på denne platform.")
        }
        ("da", MessageKey::WindowOpacityFailed) => {
            Some("Vinduesgennemsigtighed kunne ikke ændres: {error}")
        }
        ("fi", MessageKey::WindowOpacitySupported) => {
            Some("Ikkunan läpinäkyvyyttä tuetaan Windowsissa.")
        }
        ("fi", MessageKey::WindowOpacityUnsupported) => {
            Some("Ikkunan läpinäkyvyyttä ei tueta tällä alustalla.")
        }
        ("fi", MessageKey::WindowOpacityFailed) => {
            Some("Ikkunan läpinäkyvyyden muuttaminen epäonnistui: {error}")
        }
        ("no", MessageKey::WindowOpacitySupported) => Some("Vindusopasitet støttes i Windows."),
        ("no", MessageKey::WindowOpacityUnsupported) => {
            Some("Vindusopasitet støttes ikke på denne plattformen.")
        }
        ("no", MessageKey::WindowOpacityFailed) => {
            Some("Kunne ikke endre vindusopasiteten: {error}")
        }
        ("zh-TW", MessageKey::WindowOpacitySupported) => Some("Windows 支援視窗透明度。"),
        ("zh-TW", MessageKey::WindowOpacityUnsupported) => Some("此平台不支援視窗透明度。"),
        ("zh-TW", MessageKey::WindowOpacityFailed) => Some("無法變更視窗透明度：{error}"),
        _ => None,
    }
}

/// Interpolates named values without treating the translated string as a format program.
pub fn text(locale: &str, key: MessageKey, args: &[(&str, &str)]) -> String {
    let locale = resolve_locale(locale);
    let result = override_text(locale, key)
        .map(str::to_owned)
        .unwrap_or_else(|| literal(locale, english(key)));
    interpolate(result, args)
}

fn interpolate(mut result: String, args: &[(&str, &str)]) -> String {
    for (name, value) in args {
        result = result.replace(&format!("{{{name}}}"), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_resolution_handles_regions_and_unsupported_languages() {
        assert_eq!(resolve_locale("ES_mx"), "es");
        assert_eq!(resolve_locale("zh-TW"), "zh-TW");
        assert_eq!(resolve_locale("xx-YY"), "en");
    }

    #[test]
    fn literal_catalog_covers_every_locale_and_interpolates_placeholders() {
        assert_eq!(text("fr", MessageKey::AiHelp, &[]), "Aide IA");
        assert_eq!(
            text("fr", MessageKey::ImportConfirm, &[("profile", "Work")]),
            "Import into Work"
        );
        assert_eq!(text("xx", MessageKey::Cancel, &[]), "Cancel");
        assert_eq!(literal("es", "Cancel"), "Cancelar");
        assert_eq!(literal("es", "Commands"), literal("es", "Command"));
        assert_eq!(
            literal("es", "Install AI coding tools"),
            literal("es", "Open vibe install guide")
        );
        assert_eq!(
            formatted_literal(
                "fr",
                "Current app language: {language}",
                &[("language", "Français")],
            ),
            "Langue actuelle de l'application : Français"
        );
        assert_eq!(
            LOCALE_INFO.iter().map(|info| info.code).collect::<Vec<_>>(),
            SUPPORTED_LOCALES
        );
        assert!(locale_info("ar").right_to_left);
        for (english, translations) in &literal_catalog().messages {
            assert_eq!(translations.len(), SUPPORTED_LOCALES.len(), "{english}");
            assert_eq!(
                translations.get("en").map(String::as_str),
                Some(english.as_str())
            );
            for locale in SUPPORTED_LOCALES {
                assert!(translations.contains_key(locale), "{locale}: {english}");
            }
        }
    }

    #[test]
    fn account_sign_in_copy_is_localized_in_every_shipped_locale() {
        let strings = [
            "ButtonsCLI account",
            "No account signed in",
            "Sign-in manages your ButtonsCLI account session and checks server access. Local terminals and provider settings work without an account.",
            "Email address",
            "Enter the code sent to {email}.",
            "Continue",
            "Send code",
            "Sign-in code sent.",
            "Signed in to ButtonsCLI.",
            "Could not send a sign-in code.",
            "Could not verify that code.",
            "Sign out",
        ];
        for locale in SUPPORTED_LOCALES {
            for english in strings {
                let translated = literal(locale, english);
                assert!(!translated.is_empty(), "missing {english} in {locale}");
                if locale != "en" {
                    assert_ne!(translated, english, "untranslated {english} in {locale}");
                }
            }
        }
        assert_eq!(
            formatted_literal(
                "es",
                "Enter the code sent to {email}.",
                &[("email", "a@test")]
            ),
            "Ingrese el código enviado a a@test."
        );
    }

    #[test]
    fn quick_secrets_safety_and_recovery_copy_is_localized_in_every_shipped_locale() {
        let strings = [
            "No recovery. If you forget this passphrase, delete the vault and create a new one.",
            "Choose a ready terminal. The shell may echo pasted text into terminal output.",
            "Forget vault",
            "Delete vault and secrets",
            "This permanently deletes every saved secret in the active profile.",
            "Use a passphrase with at least 12 characters.",
            "The passphrase did not unlock this vault.",
            "Quick Secrets could not complete this action.",
        ];
        for locale in SUPPORTED_LOCALES {
            for english in strings {
                let translated = literal(locale, english);
                assert!(!translated.is_empty(), "missing {english} in {locale}");
                if locale != "en" {
                    assert_ne!(translated, english, "untranslated {english} in {locale}");
                }
            }
        }
    }

    #[test]
    fn every_key_has_english_text_in_every_supported_locale() {
        for locale in SUPPORTED_LOCALES {
            for key in MessageKey::ALL {
                assert!(!text(locale, key, &[]).is_empty());
            }
        }
    }

    #[test]
    fn legacy_shader_warning_has_a_translation_for_every_supported_locale() {
        let english = text("en", MessageKey::LegacyShaderNotRendered, &[]);
        for locale in SUPPORTED_LOCALES
            .into_iter()
            .filter(|locale| *locale != "en")
        {
            assert_ne!(
                text(locale, MessageKey::LegacyShaderNotRendered, &[]),
                english,
                "{locale}"
            );
        }
    }
}
