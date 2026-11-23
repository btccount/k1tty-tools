"use strict";

const $ = (s, root = document) => root.querySelector(s);
const $$ = (s, root = document) => Array.from(root.querySelectorAll(s));
const esc = (s) => {
  const d = document.createElement("div");
  d.textContent = s ?? "";
  return d.innerHTML;
};

function ico(name, cls = "") {
  return `<svg class="ico${cls ? " " + cls : ""}"><use href="#i-${name}"/></svg>`;
}

const I18N = {
  ru: {
    notices_btn: "Уведомления",
    notices_title: "Системные уведомления",
    notices_type: "Тип уведомления",
    notices_text: "Текст уведомления",
    notices_text_ph: "Введите текст уведомления",
    notices_extra: "Дополнение к уведомлению",
    notices_extra_ph: "Дополнительный текст, если нужен",
    notices_append_hint: "Уведомление будет добавлено в конец диалога.",
    notices_info: "Информация",
    notices_notice: "Уведомление",
    notices_suggestion: "Подсказка",
    notices_warning: "Предупреждение",
    notices_safety_classifier: "Шаблон Safety Classifier",
    notices_safety_label: "Уведомление безопасности (Safety Classifier)",
    notices_count: "Уведомлений в диалоге: {n}",
    notices_none: "В этом диалоге нет уведомлений",
    notices_delete_hint: "Удаление охватывает весь диалог, включая ещё не загруженные уведомления.",
    notices_delete_all: "Удалить все",
    notices_add: "Добавить",
    notices_adding: "Добавление",
    notices_added: "Уведомление добавлено",
    notices_deleted: "Удалено уведомлений: {n}",
    notices_loading: "Читаем уведомления…",
    notices_enter_text: "Введите текст уведомления",
    window_minimize: "Свернуть",
    window_maximize: "Развернуть",
    window_restore: "Восстановить",
    window_close: "Закрыть",

    all: "Все",
    search: "Поиск",
    clear: "Очистить",
    overview: "Обзор",
    overview_tip: "Обзор хранилищ",
    ov_title: "Локальный архив",
    ov_sub: "Диалоги Antigravity, Antigravity CLI, Codex, Claude и OpenCode на этом диске",
    backups: "Резервные копии",
    backups_short: "Копии",
    dialog_backups: "Копии диалога",
    refresh: "Обновить",
    hide_sidebar: "Скрыть панель",
    show_sidebar: "Показать панель",
    tg_channel: "Telegram-канал",
    cat_toggle: "Котик за курсором",
    about: "О K1tty Tools",
    privacy_tip: "Скрыть названия диалогов",
    privacy_on_tip: "Показать названия диалогов",
    winter_tip: "Включить зимнюю тему",
    winter_on_tip: "Отключить зимнюю тему",
    toast_winter_on: "Зимняя тема включена",
    toast_winter_off: "Зимняя тема отключена",
    service: "Служебные",
    service_tip: "Команды, выводы и размышления",
    lock: "Забанить",
    unlock: "Разбанить",
    lock_tip: "Заблокировать чат",
    unlock_tip: "Разблокировать чат",
    storages: "Хранилища",
    recent: "Недавние",
    stat_dialogs: "Всего диалогов",
    stat_storages: "{a} из {b} хранилищ",
    stat_disk: "Объём на диске",
    stat_largest: "крупнейший: {v}",
    stat_no_files: "файлов нет",
    stat_procs: "Активные процессы",
    stat_backups: "Резервные копии",
    stat_backups_sub: "{n} копий",
    stat_backups_empty: "копий нет",
    ov_backups_title: "Резервные копии",
    auto_backup: "Автоматические копии",
    auto_backup_hint: "Перед правкой и удалением",
    auto_backup_tip: "Автоматически создавать резервную копию перед правкой и удалением",
    toast_autobk_on: "Автобекап включён",
    toast_autobk_off: "Автобекап отключён: правки не будут занимать место на диске",
    explore: "Исследовать",
    clean_all: "Очистить",
    disk_chats: "Диалоги",
    disk_backups: "Бекапы",
    disk_total: "Всего: {v}",
    bk_none_short: "нет копий",
    rep_status: "Статус",
    rep_ok: "Успешно",
    rep_integrity: "Целостность БД",
    rep_backup: "Резервная копия",
    rep_backup_off: "Не создавалась (автобекап выкл.)",
    rep_dialog: "Диалог",
    rep_step: "Позиция",
    rep_removed: "Удалено",
    rep_freed: "Освобождено",
    rep_patched: "Изменено записей",
    rep_turns: "Обработано ходов",
    rep_files_removed: "Удалено файлов",
    rep_copy_path: "Копировать путь",
    rep_path_copied: "Путь скопирован",
    reveal_in_explorer: "Открыть в проводнике",
    reveal_desktop_only: "Открытие файлов доступно в десктопном приложении",
    tm_copy: "Копировать",
    tm_cut: "Вырезать",
    tm_copy_tex: "Копировать формулу (TeX)",
    tm_copy_code: "Копировать код",
    tm_copy_link: "Копировать ссылку",
    tm_copy_path: "Копировать путь",
    tm_copy_cell: "Копировать ячейку",
    tm_copy_table: "Копировать таблицу (Markdown)",
    tm_copy_msg: "Копировать сообщение (Markdown)",
    tm_select_msg: "Выделить сообщение",
    tm_select_all: "Выделить всё",
    tm_copied: "Скопировано",
    stat_busy_files: "есть занятые файлы",
    stat_no_procs: "приложения не запущены",
    recent_of: "{a} из {b}",
    no_dialogs: "Нет диалогов",
    no_dialogs_yet: "Диалогов пока нет",
    no_messages: "Сообщений нет",
    no_backups: "Копий пока нет",
    no_preview: "Нет записей для предпросмотра",
    status_ready: "Готов",
    status_not_found: "Не найден",
    status_busy: "Файлы заняты",
    status_active: "Активен",
    status_ro: "Только чтение",
    pill_running: "{name} открыт",
    pill_blocked: "{name} занят",
    pill_live_unlocked: "Live Unlock",
    running_guard_banner: "Файл диалога занят {name} · Закройте этот диалог или приложение для редактирования",
    running_guard_unlock_banner: "Файл диалога занят {name} · Можно разблокировать БД для WAL-правки или перезапустить Hub",
    live_unlocked_banner: "Live Unlock активен · WAL-запись в БД {name} открыта. После правки переоткройте диалог или перезапустите Hub",
    unlock_db_btn: "Разблокировать БД",
    relock_db_btn: "Вернуть защиту",
    restart_hub_btn: "Перезапустить Hub",
    unlocking_db: "Разблокировка...",
    restarting_hub: "Перезапуск Hub...",
    toast_db_unlocked: "БД разблокирована (Live Unlock · WAL)",
    toast_db_relocked: "Защита присутствия возвращена",
    toast_hub_restarted: "Hub {name} перезапущен (PID: {pids})",
    toast_hub_not_found: "Активный процесс Hub {name} не найден",
    toast_close_app_first: "Файл занят {name} · Закройте диалог или приложение",
    actions: "Действия",
    open: "Открыть",
    copy_id: "Копировать ID",
    lock_chat: "Забанить чат",
    delete_conv: "Удалить диалог",
    confirm_delete: "Подтвердить удаление",
    confirm: "Подтвердить",
    edit: "Изменить",
    delete: "Удалить",
    cut_above: "Выше",
    cut_below: "Обрезать",
    cut_head_title: "Удалить всё выше",
    cut_tail_title: "Обрезать после",
    cut: "Обрезка",
    cancel: "Отмена",
    save: "Сохранить",
    saving: "Сохранение",
    deleting: "Удаление",
    loading: "Загрузка",
    restoring: "Откат",
    restore: "Восстановить",
    close: "Закрыть",
    done: "Готово",
    saved: "Сохранено",
    yes: "Да",
    no: "Нет",
    unlocking: "Снятие",
    locking: "Блокировка",
    lock_modal_title: "Заблокировать чат",
    lock_reason_lbl: "Текст ошибки",
    lock_reason_ph: "Причина блокировки",
    lock_code_lbl: "Код политики",
    lock_apply: "Заблокировать",
    custom_reason: "Своя причина",
    scroll_bottom: "Вниз",
    start_line: "Начало диалога",
    load_more: "Ранние сообщения",
    you: "Вы",
    tool: "Инструмент",
    system: "Система",
    error: "Ошибка",
    empty: "Пусто",
    transcript_only: "только в транскрипте",
    show_more: "Показать полностью",
    collapse: "Свернуть",
    copy: "Копировать",
    copied: "Скопировано",
    stats_items: "записей",
    stats_lines: "строк",
    stats_size: "объём",
    calc: "расчёт",
    backup_note: "Копия:",
    leaks_found: "Найдены следы ({n}):",
    leaks_none: "Следов удалённого текста не найдено",
    welcome_title: "Добро пожаловать в K1tty Tools",
    welcome_desc: "Редактируй и чисти чаты, снимай баны и откатывай историю в пару кликов",
    welcome_lang_label: "Язык интерфейса",
    welcome_skip: "Пропустить",
    welcome_choose_look: "Выберите оформление",
    welcome_choose_look_sub: "Классические и уютные темы с пиксельными котиками",
    welcome_back: "Назад",
    welcome_start: "Начать работу",
    continue: "Продолжить",
    select_chats_tip: "Выделить несколько чатов",
    select_chats: "Выбрать несколько",
    select_all: "Выбрать все",
    deselect_all: "Снять выбор",
    selected_n: "Выбрано: {n}",
    export_short: "Экспорт",
    batch_exported: "Экспортировано чатов: {n}",
    batch_deleted: "Удалено чатов: {n}",
    batch_skipped_locked: "Пропущено занятых чатов: {n}",
    fulltext_search_tip: "Полнотекстовый поиск по содержимому сообщений",
    fulltext_searching: "Ищем по содержимому…",
    fulltext_hint: "Введите минимум 2 символа для поиска по тексту чатов",
    fulltext_hits: "совпадений: {n}",
    find_in_chat_tip: "Поиск по диалогу (Ctrl+F)",
    find_in_chat_ph: "Поиск по всему диалогу",
    prev_match: "Предыдущее совпадение",
    next_match: "Следующее совпадение",
    find_no_matches: "Нет совпадений",
    slim_tip: "Очистить thinking и тяжелые шаги",
    slim_title: "Оптимизация контекста",
    slim_opt_thinking: "Удалить размышления (thinking)",
    slim_opt_tools: "Сократить тяжелые выводы инструментов",
    slim_threshold_lbl: "Порог размера вывода",
    slim_heaviest_lbl: "Самые тяжелые шаги",
    slim_apply: "Очистить",
    slim_unsupported: "Для этого провайдера доступен просмотр веса контекста; очистка thinking и выводов поддерживается для Claude, Codex и OpenCode.",
    slim_nothing: "Нет шагов, подходящих под выбранные условия очистки.",
    slim_savings_txt: "Будет обработано шагов: {n} · Освободится {size} (~{tok} ток.)",
    slim_done: "Очищено: освобождено {size} (~{tok} ток.)",
    slim_total_tok: "Всего в контексте",
    slim_think_tok: "Thinking",
    slim_tool_tok: "Инструменты",
    tok_unit: "ток.",
    pin_chat: "Закрепить",
    unpin_chat: "Открепить",
    toast_pinned: "Чат закреплен",
    toast_unpinned: "Чат откреплен",
    palette_tip: "Палитра команд (Ctrl+K)",
    palette_ph: "Найти чат, сменить тему или выполнить действие",
    palette_chats: "Диалоги",
    palette_actions: "Действия",
    palette_themes: "Темы оформления",
    palette_empty: "Ничего не найдено",
    palette_open_settings: "Открыть настройки",
    palette_toggle_fulltext: "Полнотекстовый поиск по всем чатам",
    palette_toggle_select: "Режим выбора нескольких чатов",
    palette_toggle_privacy: "Переключить режим приватности",
    palette_open_slim: "Оптимизация контекста и thinking",
    welcome_sign_prefix: "с любовью от",
    welcome_sign_names: "quesada & k1ttyf <3",
    unlocker_default: "Этот диалог заблокирован. Хотите снять блокировку?",
    toast_reconnected: "Соединение восстановлено",
    toast_reconnecting: "Переподключение",
    toast_id_copied: "ID скопирован",
    toast_conv_deleted: "Диалог удалён",
    toast_undo: "Отменить",
    toast_conv_restored: "Диалог восстановлен",
    toast_empty_text: "Текст пустой",
    toast_unlocked: "Чат разблокирован",
    toast_codex_only: "Блокировка поддерживается для Codex",
    toast_enter_reason: "Укажите причину блокировки",
    toast_locked: "Чат заблокирован",
    toast_step_deleted: "Шаг удалён",
    toast_restored: "Восстановлено",
    err_no_conn: "Нет соединения с сервером",
    err_conn_drop: "Соединение разорвано",
    err_conn_lost: "Соединение потеряно",
    err_server: "Ошибка сервера",
    err_timeout: "Таймаут операции: {op}",
    err_op: "Ошибка операции",
    phase_running: "Выполнение",
    phase_start: "Подготовка",
    phase_backup: "Резервная копия",
    phase_cut: "Удаление",
    phase_edit: "Сохранение",
    phase_lock: "Блокировка",
    phase_unlock: "Разблокировка",
    phase_restore: "Восстановление",
    phase_leak: "Проверка следов",
    phase_read: "Чтение истории",
    phase_write: "Запись диалога",
    phase_done: "Готово",
    import_k1tty: "Импорт .k1tty",
    import_short: "Импорт",
    export_k1tty: "Экспорт .k1tty",
    convert_chat: "Конвертировать",
    convert_title: "Конвертировать чат",
    compression_preset: "Сжатие",
    preset_fast: "Быстрый",
    preset_balanced: "Баланс",
    preset_ultra: "Ультра",
    include_service_steps: "Включить служебные шаги",
    export_btn: "Экспортировать",
    exporting: "Упаковка",
    export_ready: "Архив .k1tty готов",
    download_again: "Скачать ещё раз",
    target_provider: "Целевая среда",
    chat_title_lbl: "Название чата",
    convert_btn: "Конвертировать",
    converting: "Конвертация",
    toast_converted: "Чат конвертирован в {prov}",
    drop_k1tty_hint: "Выберите или перетащите файл .k1tty",
    import_btn: "Импортировать",
    importing: "Импорт",
    toast_imported: "Чат импортирован в {prov}",
    import_failed: "Не удалось импортировать чат",
    inspecting: "Проверка архива",
    pack_entries: "записей",
    pack_saved: "сжатие",
    pack_time: "мс",
    inspect_bk: "Исследовать",
    bk_research_banner: "Режим исследования копии · Только чтение",
    exit_research: "Выйти",
    delete_all_backups: "Удалить все копии",
    delete_all_bk_title: "Удалить все резервные копии",
    delete_all_bk_warn: "Будут безвозвратно удалены все резервные копии со всех хранилищ:",
    delete_all_confirm: "Удалить всё",
    deleting_all: "Очистка",
    bk_count_lbl: "{n} коп.",
    bk_total_lbl: "Итого к удалению",
    toast_bk_deleted: "Копия удалена · освобождено {size}",
    toast_all_bk_deleted: "Удалено {n} копий · освобождено {size}",
    bk_cleanup_done: "Очистка резервных копий завершена",
    bk_deleted_count: "Удалено копий",
    bk_freed_space: "Освобождено",
    u_b: "Б",
    u_kb: "КБ",
    u_mb: "МБ",
    u_gb: "ГБ",
    u_steps: "ш.",
    u_files: "ф.",
    date_locale: "ru-RU",
  },
  uk: {
    notices_btn: "Сповіщення",
    notices_title: "Системні сповіщення",
    notices_type: "Тип сповіщення",
    notices_text: "Текст сповіщення",
    notices_text_ph: "Введіть текст сповіщення",
    notices_extra: "Доповнення до сповіщення",
    notices_extra_ph: "Додатковий текст, якщо потрібен",
    notices_append_hint: "Сповіщення буде додано в кінець діалогу.",
    notices_info: "Інформація",
    notices_notice: "Сповіщення",
    notices_suggestion: "Підказка",
    notices_warning: "Попередження",
    notices_safety_classifier: "Шаблон Safety Classifier",
    notices_safety_label: "Сповіщення безпеки (Safety Classifier)",
    notices_count: "Сповіщень у діалозі: {n}",
    notices_none: "У цьому діалозі немає сповіщень",
    notices_delete_hint: "Видалення охоплює весь діалог, включно з ще не завантаженими сповіщеннями.",
    notices_delete_all: "Видалити всі",
    notices_add: "Додати",
    notices_adding: "Додавання",
    notices_added: "Сповіщення додано",
    notices_deleted: "Видалено сповіщень: {n}",
    notices_loading: "Читаємо сповіщення…",
    notices_enter_text: "Введіть текст сповіщення",
    window_minimize: "Згорнути",
    window_maximize: "Розгорнути",
    window_restore: "Відновити",
    window_close: "Закрити",

    all: "Усі",
    search: "Пошук",
    clear: "Очистити",
    overview: "Огляд",
    overview_tip: "Огляд сховищ",
    ov_title: "Локальний архів",
    ov_sub: "Діалоги Antigravity, Antigravity CLI, Codex, Claude та OpenCode на цьому диску",
    backups: "Резервні копії",
    backups_short: "Копії",
    dialog_backups: "Копії діалогу",
    refresh: "Оновити",
    hide_sidebar: "Сховати панель",
    show_sidebar: "Показати панель",
    tg_channel: "Telegram-канал",
    cat_toggle: "Котик за курсором",
    about: "Про K1tty Tools",
    privacy_tip: "Приховати назви діалогів",
    privacy_on_tip: "Показати назви діалогів",
    winter_tip: "Увімкнути зимову тему",
    winter_on_tip: "Вимкнути зимову тему",
    toast_winter_on: "Зимову тему увімкнено",
    toast_winter_off: "Зимову тему вимкнено",
    service: "Службові",
    service_tip: "Команди, виводи та міркування",
    lock: "Забанити",
    unlock: "Розбанити",
    lock_tip: "Заблокувати чат",
    unlock_tip: "Розблокувати чат",
    storages: "Сховища",
    recent: "Останні",
    stat_dialogs: "Усього діалогів",
    stat_storages: "{a} з {b} сховищ",
    stat_disk: "Обсяг на диску",
    stat_largest: "найбільший: {v}",
    stat_no_files: "файлів немає",
    stat_procs: "Активні процеси",
    stat_backups: "Резервні копії",
    stat_backups_sub: "{n} копій",
    stat_backups_empty: "копій немає",
    ov_backups_title: "Резервні копії",
    auto_backup: "Автоматичні копії",
    auto_backup_hint: "Перед правкою та видаленням",
    auto_backup_tip: "Автоматично створювати резервну копію перед правкою та видаленням",
    toast_autobk_on: "Автобекап увімкнено",
    toast_autobk_off: "Автобекап вимкнено: правки не займатимуть місце на диску",
    explore: "Дослідити",
    clean_all: "Очистити",
    disk_chats: "Діалоги",
    disk_backups: "Бекапи",
    disk_total: "Усього: {v}",
    bk_none_short: "немає копій",
    rep_status: "Статус",
    rep_ok: "Успішно",
    rep_integrity: "Цілісність БД",
    rep_backup: "Резервна копія",
    rep_backup_off: "Не створювалась (автобекап вимк.)",
    rep_dialog: "Діалог",
    rep_step: "Позиція",
    rep_removed: "Видалено",
    rep_freed: "Звільнено",
    rep_patched: "Змінено записів",
    rep_turns: "Оброблено ходів",
    rep_files_removed: "Видалено файлів",
    rep_copy_path: "Копіювати шлях",
    rep_path_copied: "Шлях скопійовано",
    reveal_in_explorer: "Відкрити у провіднику",
    reveal_desktop_only: "Відкриття файлів доступне у десктопному застосунку",
    tm_copy: "Копіювати",
    tm_cut: "Вирізати",
    tm_copy_tex: "Копіювати формулу (TeX)",
    tm_copy_code: "Копіювати код",
    tm_copy_link: "Копіювати посилання",
    tm_copy_path: "Копіювати шлях",
    tm_copy_cell: "Копіювати клітинку",
    tm_copy_table: "Копіювати таблицю (Markdown)",
    tm_copy_msg: "Копіювати повідомлення (Markdown)",
    tm_select_msg: "Виділити повідомлення",
    tm_select_all: "Виділити все",
    tm_copied: "Скопійовано",
    stat_busy_files: "є зайняті файли",
    stat_no_procs: "застосунки не запущено",
    recent_of: "{a} з {b}",
    no_dialogs: "Немає діалогів",
    no_dialogs_yet: "Діалогів поки немає",
    no_messages: "Повідомлень немає",
    no_backups: "Копій поки немає",
    no_preview: "Немає записів для перегляду",
    status_ready: "Готовий",
    status_not_found: "Не знайдено",
    status_busy: "Файли зайняті",
    status_active: "Активний",
    status_ro: "Лише читання",
    pill_running: "{name} відкрито",
    pill_blocked: "{name} зайнятий",
    pill_live_unlocked: "Live Unlock",
    running_guard_banner: "Файл діалогу зайнятий {name} · Закрийте цей діалог або застосунок для редагування",
    running_guard_unlock_banner: "Файл діалогу зайнятий {name} · Можна розблокувати БД для WAL-правки або перезапустити Hub",
    live_unlocked_banner: "Live Unlock активний · WAL-запис у БД {name} відкрито. Після правки перевідкрийте діалог або перезапустіть Hub",
    unlock_db_btn: "Розблокувати БД",
    relock_db_btn: "Повернути захист",
    restart_hub_btn: "Перезапустити Hub",
    unlocking_db: "Розблокування...",
    restarting_hub: "Перезапуск Hub...",
    toast_db_unlocked: "БД розблоковано (Live Unlock · WAL)",
    toast_db_relocked: "Захист присутності повернуто",
    toast_hub_restarted: "Hub {name} перезапущено (PID: {pids})",
    toast_hub_not_found: "Активний процес Hub {name} не знайдено",
    toast_close_app_first: "Файл зайнятий {name} · Закрийте діалог або застосунок",
    actions: "Дії",
    open: "Відкрити",
    copy_id: "Копіювати ID",
    lock_chat: "Забанити чат",
    delete_conv: "Видалити діалог",
    confirm_delete: "Підтвердити видалення",
    confirm: "Підтвердити",
    edit: "Змінити",
    delete: "Видалити",
    cut_above: "Вище",
    cut_below: "Обрізати",
    cut_head_title: "Видалити все вище",
    cut_tail_title: "Обрізати після",
    cut: "Обрізка",
    cancel: "Скасувати",
    save: "Зберегти",
    saving: "Збереження",
    deleting: "Видалення",
    loading: "Завантаження",
    restoring: "Відкат",
    restore: "Відновити",
    close: "Закрити",
    done: "Готово",
    saved: "Збережено",
    yes: "Так",
    no: "Ні",
    unlocking: "Зняття",
    locking: "Блокування",
    lock_modal_title: "Заблокувати чат",
    lock_reason_lbl: "Текст помилки",
    lock_reason_ph: "Причина блокування",
    lock_code_lbl: "Код політики",
    lock_apply: "Заблокувати",
    custom_reason: "Своя причина",
    scroll_bottom: "Донизу",
    start_line: "Початок діалогу",
    load_more: "Раніші повідомлення",
    you: "Ви",
    tool: "Інструмент",
    system: "Система",
    error: "Помилка",
    empty: "Порожньо",
    transcript_only: "лише в транскрипті",
    show_more: "Показати повністю",
    collapse: "Згорнути",
    copy: "Копіювати",
    copied: "Скопійовано",
    stats_items: "записів",
    stats_lines: "рядків",
    stats_size: "обсяг",
    calc: "розрахунок",
    backup_note: "Копія:",
    leaks_found: "Знайдено сліди ({n}):",
    leaks_none: "Слідів видаленого тексту не знайдено",
    welcome_title: "Ласкаво просимо до K1tty Tools",
    welcome_desc: "Редагуй і чисти чати, знімай бани та відкочуй історію в пару кліків",
    welcome_lang_label: "Мова інтерфейсу",
    welcome_skip: "Пропустити",
    welcome_choose_look: "Оберіть оформлення",
    welcome_choose_look_sub: "Класичні та затишні теми з піксельними котиками",
    welcome_back: "Назад",
    welcome_start: "Почати роботу",
    continue: "Продовжити",
    select_chats_tip: "Виділити кілька чатів",
    select_chats: "Обрати кілька",
    select_all: "Обрати всі",
    deselect_all: "Зняти вибір",
    selected_n: "Обрано: {n}",
    export_short: "Експорт",
    batch_exported: "Експортовано чатів: {n}",
    batch_deleted: "Видалено чатів: {n}",
    batch_skipped_locked: "Пропущено зайнятих чатів: {n}",
    fulltext_search_tip: "Повнотекстовий пошук за вмістом повідомлень",
    fulltext_searching: "Шукаємо за вмістом…",
    fulltext_hint: "Введіть щонайменше 2 символи для пошуку по тексту чатів",
    fulltext_hits: "збігів: {n}",
    find_in_chat_tip: "Пошук у діалозі (Ctrl+F)",
    find_in_chat_ph: "Пошук по всьому діалогу",
    prev_match: "Попередній збіг",
    next_match: "Наступний збіг",
    find_no_matches: "Немає збігів",
    slim_tip: "Очистити thinking і важкі кроки",
    slim_title: "Оптимізація контексту",
    slim_opt_thinking: "Видалити міркування (thinking)",
    slim_opt_tools: "Скоротити важкі виводи інструментів",
    slim_threshold_lbl: "Поріг розміру виводу",
    slim_heaviest_lbl: "Найважчі кроки",
    slim_apply: "Очистити",
    slim_unsupported: "Для цього провайдера доступний перегляд ваги контексту; очищення thinking і виводів підтримується для Claude, Codex і OpenCode.",
    slim_nothing: "Немає кроків за обраними параметрами очищення.",
    slim_savings_txt: "Буде оброблено кроків: {n} · Звільниться {size} (~{tok} ток.)",
    slim_done: "Очищено: звільнено {size} (~{tok} ток.)",
    slim_total_tok: "Усього в контексті",
    slim_think_tok: "Thinking",
    slim_tool_tok: "Інструменти",
    tok_unit: "ток.",
    pin_chat: "Закріпити",
    unpin_chat: "Відкріпити",
    toast_pinned: "Чат закріплено",
    toast_unpinned: "Чат відкріплено",
    palette_tip: "Палітра команд (Ctrl+K)",
    palette_ph: "Знайти чат, змінити тему або виконати дію",
    palette_chats: "Діалоги",
    palette_actions: "Дії",
    palette_themes: "Теми оформлення",
    palette_empty: "Нічого не знайдено",
    palette_open_settings: "Відкрити налаштування",
    palette_toggle_fulltext: "Повнотекстовий пошук по всіх чатах",
    palette_toggle_select: "Режим вибору кількох чатів",
    palette_toggle_privacy: "Перемкнути режим приватності",
    palette_open_slim: "Оптимізація контексту та thinking",
    welcome_sign_prefix: "з любов'ю від",
    welcome_sign_names: "quesada & k1ttyf <3",
    unlocker_default: "Цей діалог заблоковано. Хочете зняти блокування?",
    toast_reconnected: "З'єднання відновлено",
    toast_reconnecting: "Перепідключення",
    toast_id_copied: "ID скопійовано",
    toast_conv_deleted: "Діалог видалено",
    toast_undo: "Скасувати",
    toast_conv_restored: "Діалог відновлено",
    toast_empty_text: "Текст порожній",
    toast_unlocked: "Чат розблоковано",
    toast_codex_only: "Блокування підтримується для Codex",
    toast_enter_reason: "Вкажіть причину блокування",
    toast_locked: "Чат заблоковано",
    toast_step_deleted: "Крок видалено",
    toast_restored: "Відновлено",
    err_no_conn: "Немає з'єднання з сервером",
    err_conn_drop: "З'єднання розірвано",
    err_conn_lost: "З'єднання втрачено",
    err_server: "Помилка сервера",
    err_timeout: "Таймаут операції: {op}",
    err_op: "Помилка операції",
    phase_running: "Виконання",
    phase_start: "Підготовка",
    phase_backup: "Резервна копія",
    phase_cut: "Видалення",
    phase_edit: "Збереження",
    phase_lock: "Блокування",
    phase_unlock: "Розблокування",
    phase_restore: "Відновлення",
    phase_leak: "Перевірка слідів",
    phase_read: "Читання історії",
    phase_write: "Запис діалогу",
    phase_done: "Готово",
    import_k1tty: "Імпорт .k1tty",
    import_short: "Імпорт",
    export_k1tty: "Експорт .k1tty",
    convert_chat: "Конвертувати",
    convert_title: "Конвертувати чат",
    compression_preset: "Стиснення",
    preset_fast: "Швидкий",
    preset_balanced: "Баланс",
    preset_ultra: "Ультра",
    include_service_steps: "Додати службові кроки",
    export_btn: "Експортувати",
    exporting: "Пакування",
    export_ready: "Архів .k1tty готовий",
    download_again: "Завантажити ще раз",
    target_provider: "Цільове середовище",
    chat_title_lbl: "Назва чату",
    convert_btn: "Конвертувати",
    converting: "Конвертація",
    toast_converted: "Чат конвертовано в {prov}",
    drop_k1tty_hint: "Оберіть або перетягніть файл .k1tty",
    import_btn: "Імпортувати",
    importing: "Імпорт",
    toast_imported: "Чат імпортовано в {prov}",
    import_failed: "Не вдалося імпортувати чат",
    inspecting: "Перевірка архіву",
    pack_entries: "записів",
    pack_saved: "стиснення",
    pack_time: "мс",
    inspect_bk: "Дослідити",
    bk_research_banner: "Режим дослідження копії · Лише читання",
    exit_research: "Вийти",
    delete_all_backups: "Видалити всі копії",
    delete_all_bk_title: "Видалити всі резервні копії",
    delete_all_bk_warn: "Усі резервні копії з усіх сховищ буде безповоротно видалено:",
    delete_all_confirm: "Видалити все",
    deleting_all: "Очищення",
    bk_count_lbl: "{n} коп.",
    bk_total_lbl: "Разом до видалення",
    toast_bk_deleted: "Копію видалено · звільнено {size}",
    toast_all_bk_deleted: "Видалено {n} копій · звільнено {size}",
    bk_cleanup_done: "Очищення резервних копій завершено",
    bk_deleted_count: "Видалено копій",
    bk_freed_space: "Звільнено",
    u_b: "Б",
    u_kb: "КБ",
    u_mb: "МБ",
    u_gb: "ГБ",
    u_steps: "кр.",
    u_files: "ф.",
    date_locale: "uk-UA",
  },
  en: {
    notices_btn: "Notices",
    notices_title: "System notices",
    notices_type: "Notice type",
    notices_text: "Notice text",
    notices_text_ph: "Enter the notice text",
    notices_extra: "Additional text",
    notices_extra_ph: "Optional text to add to the notice",
    notices_append_hint: "The notice will be added at the end of the conversation.",
    notices_info: "Information",
    notices_notice: "Notice",
    notices_suggestion: "Suggestion",
    notices_warning: "Warning",
    notices_safety_classifier: "Safety Classifier template",
    notices_safety_label: "Safety Classifier notice",
    notices_count: "Notices in this conversation: {n}",
    notices_none: "No notices in this conversation",
    notices_delete_hint: "Removal covers the whole conversation, including notices that have not loaded yet.",
    notices_delete_all: "Delete all",
    notices_add: "Add notice",
    notices_adding: "Adding",
    notices_added: "Notice added",
    notices_deleted: "Notices removed: {n}",
    notices_loading: "Reading notices…",
    notices_enter_text: "Enter the notice text",
    window_minimize: "Minimize",
    window_maximize: "Maximize",
    window_restore: "Restore",
    window_close: "Close",

    all: "All",
    search: "Search",
    clear: "Clear",
    overview: "Overview",
    overview_tip: "Storage overview",
    ov_title: "Local archive",
    ov_sub: "Antigravity, Antigravity CLI, Codex, Claude, and OpenCode dialogues on this disk",
    backups: "Backups",
    backups_short: "Backups",
    dialog_backups: "Dialogue backups",
    refresh: "Refresh",
    hide_sidebar: "Hide sidebar",
    show_sidebar: "Show sidebar",
    tg_channel: "Telegram channel",
    cat_toggle: "Cursor cat",
    about: "About K1tty Tools",
    privacy_tip: "Hide dialogue titles",
    privacy_on_tip: "Show dialogue titles",
    winter_tip: "Enable winter theme",
    winter_on_tip: "Disable winter theme",
    toast_winter_on: "Winter theme enabled",
    toast_winter_off: "Winter theme disabled",
    service: "Service",
    service_tip: "Commands, outputs, and reasoning",
    lock: "Lock",
    unlock: "Unlock",
    lock_tip: "Lock chat",
    unlock_tip: "Unlock chat",
    storages: "Storages",
    recent: "Recent",
    stat_dialogs: "Total dialogues",
    stat_storages: "{a} of {b} storages",
    stat_disk: "Disk usage",
    stat_largest: "largest: {v}",
    stat_no_files: "no files",
    stat_procs: "Active processes",
    stat_backups: "Backups",
    stat_backups_sub: "{n} copies",
    stat_backups_empty: "no backups",
    ov_backups_title: "Backups",
    auto_backup: "Automatic backups",
    auto_backup_hint: "Before editing or deleting",
    auto_backup_tip: "Automatically create a backup before editing or deleting",
    toast_autobk_on: "Auto-backup enabled",
    toast_autobk_off: "Auto-backup disabled: edits will not use extra disk space",
    explore: "Explore",
    clean_all: "Clean all",
    disk_chats: "Dialogues",
    disk_backups: "Backups",
    disk_total: "Total: {v}",
    bk_none_short: "no backups",
    rep_status: "Status",
    rep_ok: "Completed",
    rep_integrity: "Integrity check",
    rep_backup: "Backup",
    rep_backup_off: "Skipped (auto-backup off)",
    rep_dialog: "Dialogue",
    rep_step: "Position",
    rep_removed: "Removed",
    rep_freed: "Freed",
    rep_patched: "Modified records",
    rep_turns: "Updated turns",
    rep_files_removed: "Removed files",
    rep_copy_path: "Copy path",
    rep_path_copied: "Path copied",
    reveal_in_explorer: "Reveal in Explorer",
    reveal_desktop_only: "Opening files is available in the desktop app",
    tm_copy: "Copy",
    tm_cut: "Cut",
    tm_copy_tex: "Copy formula (TeX)",
    tm_copy_code: "Copy code",
    tm_copy_link: "Copy link",
    tm_copy_path: "Copy path",
    tm_copy_cell: "Copy cell",
    tm_copy_table: "Copy table (Markdown)",
    tm_copy_msg: "Copy message (Markdown)",
    tm_select_msg: "Select message",
    tm_select_all: "Select all",
    tm_copied: "Copied",
    stat_busy_files: "files locked",
    stat_no_procs: "no running apps",
    recent_of: "{a} of {b}",
    no_dialogs: "No dialogues",
    no_dialogs_yet: "No dialogues yet",
    no_messages: "No messages",
    no_backups: "No backups yet",
    no_preview: "No entries to preview",
    status_ready: "Ready",
    status_not_found: "Not found",
    status_busy: "Files locked",
    status_active: "Active",
    status_ro: "Read only",
    pill_running: "{name} open",
    pill_blocked: "{name} locked",
    pill_live_unlocked: "Live Unlock",
    running_guard_banner: "Dialogue file is in use by {name} · Close the dialogue or app to edit",
    running_guard_unlock_banner: "Dialogue file is in use by {name} · Unlock DB for WAL editing or restart the Hub",
    live_unlocked_banner: "Live Unlock active · WAL writes to {name} DB enabled. Reopen the chat or restart Hub to reload changes",
    unlock_db_btn: "Unlock DB",
    relock_db_btn: "Relock",
    restart_hub_btn: "Restart Hub",
    unlocking_db: "Unlocking...",
    restarting_hub: "Restarting Hub...",
    toast_db_unlocked: "DB unlocked (Live Unlock · WAL)",
    toast_db_relocked: "Presence lock guard restored",
    toast_hub_restarted: "{name} Hub restarted (PID: {pids})",
    toast_hub_not_found: "No running {name} Hub process found",
    toast_close_app_first: "File is in use by {name} · Close the dialogue or app first",
    actions: "Actions",
    open: "Open",
    copy_id: "Copy ID",
    lock_chat: "Lock chat",
    delete_conv: "Delete dialogue",
    confirm_delete: "Confirm delete",
    confirm: "Confirm",
    edit: "Edit",
    delete: "Delete",
    cut_above: "Above",
    cut_below: "Trim",
    cut_head_title: "Delete everything above",
    cut_tail_title: "Trim after",
    cut: "Trim",
    cancel: "Cancel",
    save: "Save",
    saving: "Saving",
    deleting: "Deleting",
    loading: "Loading",
    restoring: "Restoring",
    restore: "Restore",
    close: "Close",
    done: "Done",
    saved: "Saved",
    yes: "Yes",
    no: "No",
    unlocking: "Unlocking",
    locking: "Locking",
    lock_modal_title: "Lock chat",
    lock_reason_lbl: "Error message",
    lock_reason_ph: "Block reason",
    lock_code_lbl: "Policy code",
    lock_apply: "Lock",
    custom_reason: "Custom reason",
    scroll_bottom: "Bottom",
    start_line: "Start of dialogue",
    load_more: "Earlier messages",
    you: "You",
    tool: "Tool",
    system: "System",
    error: "Error",
    empty: "Empty",
    transcript_only: "transcript only",
    show_more: "Show full",
    collapse: "Collapse",
    copy: "Copy",
    copied: "Copied",
    stats_items: "entries",
    stats_lines: "lines",
    stats_size: "size",
    calc: "calculating",
    backup_note: "Backup:",
    leaks_found: "Leaks found ({n}):",
    leaks_none: "No traces of removed text found",
    welcome_title: "Welcome to K1tty Tools",
    welcome_desc: "Edit and clean chats, lift locks, and roll back history in a couple clicks",
    welcome_lang_label: "Interface language",
    welcome_skip: "Skip",
    welcome_choose_look: "Choose your look",
    welcome_choose_look_sub: "Classic themes and cozy pixel-art cat presets",
    welcome_back: "Back",
    welcome_start: "Get Started",
    continue: "Continue",
    select_chats_tip: "Select multiple chats",
    select_chats: "Select multiple",
    select_all: "Select all",
    deselect_all: "Clear",
    selected_n: "Selected: {n}",
    export_short: "Export",
    batch_exported: "Exported chats: {n}",
    batch_deleted: "Deleted chats: {n}",
    batch_skipped_locked: "Skipped locked chats: {n}",
    fulltext_search_tip: "Full-text search inside message contents",
    fulltext_searching: "Searching message contents…",
    fulltext_hint: "Type at least 2 characters to search inside chats",
    fulltext_hits: "matches: {n}",
    find_in_chat_tip: "Find in chat (Ctrl+F)",
    find_in_chat_ph: "Search entire conversation",
    prev_match: "Previous match",
    next_match: "Next match",
    find_no_matches: "No matches",
    slim_tip: "Strip thinking and heavy tool outputs",
    slim_title: "Context optimizer",
    slim_opt_thinking: "Remove thinking / reasoning blocks",
    slim_opt_tools: "Trim heavy tool outputs",
    slim_threshold_lbl: "Output size threshold",
    slim_heaviest_lbl: "Heaviest steps",
    slim_apply: "Clean",
    slim_unsupported: "Context size stats are available; stripping thinking and heavy outputs is supported for Claude, Codex, and OpenCode.",
    slim_nothing: "No steps match the selected cleanup options.",
    slim_savings_txt: "Steps affected: {n} · Savings {size} (~{tok} tok)",
    slim_done: "Cleaned: freed {size} (~{tok} tok)",
    slim_total_tok: "Total context",
    slim_think_tok: "Thinking",
    slim_tool_tok: "Tools",
    tok_unit: "tok",
    pin_chat: "Pin to top",
    unpin_chat: "Unpin",
    toast_pinned: "Chat pinned",
    toast_unpinned: "Chat unpinned",
    palette_tip: "Command palette (Ctrl+K)",
    palette_ph: "Jump to a chat, switch theme, or run an action",
    palette_chats: "Conversations",
    palette_actions: "Actions",
    palette_themes: "Themes",
    palette_empty: "Nothing found",
    palette_open_settings: "Open settings",
    palette_toggle_fulltext: "Toggle full-text search across chats",
    palette_toggle_select: "Toggle multi-select mode",
    palette_toggle_privacy: "Toggle privacy mode",
    palette_open_slim: "Optimize context and thinking",
    welcome_sign_prefix: "with love from",
    welcome_sign_names: "quesada & k1ttyf <3",
    unlocker_default: "This dialogue is locked. Do you want to unlock it?",
    toast_reconnected: "Connection restored",
    toast_reconnecting: "Reconnecting",
    toast_id_copied: "ID copied",
    toast_conv_deleted: "Dialogue deleted",
    toast_undo: "Undo",
    toast_conv_restored: "Dialogue restored",
    toast_empty_text: "Text is empty",
    toast_unlocked: "Chat unlocked",
    toast_codex_only: "Locking is supported for Codex",
    toast_enter_reason: "Enter a block reason",
    toast_locked: "Chat locked",
    toast_step_deleted: "Step deleted",
    toast_restored: "Restored",
    err_no_conn: "Not connected to server",
    err_conn_drop: "Connection closed",
    err_conn_lost: "Connection lost",
    err_server: "Server error",
    err_timeout: "Operation timeout: {op}",
    err_op: "Operation failed",
    phase_running: "Running",
    phase_start: "Preparing",
    phase_backup: "Creating backup",
    phase_cut: "Removing",
    phase_edit: "Saving",
    phase_lock: "Locking",
    phase_unlock: "Unlocking",
    phase_restore: "Restoring",
    phase_leak: "Checking traces",
    phase_read: "Reading history",
    phase_write: "Writing dialogue",
    phase_done: "Done",
    import_k1tty: "Import .k1tty",
    import_short: "Import",
    export_k1tty: "Export .k1tty",
    convert_chat: "Convert",
    convert_title: "Convert chat",
    compression_preset: "Compression",
    preset_fast: "Fast",
    preset_balanced: "Balanced",
    preset_ultra: "Ultra",
    include_service_steps: "Include service steps",
    export_btn: "Export",
    exporting: "Packing",
    export_ready: ".k1tty archive ready",
    download_again: "Download again",
    target_provider: "Target environment",
    chat_title_lbl: "Chat title",
    convert_btn: "Convert",
    converting: "Converting",
    toast_converted: "Chat converted to {prov}",
    drop_k1tty_hint: "Choose or drop a .k1tty file",
    import_btn: "Import",
    importing: "Importing",
    toast_imported: "Chat imported into {prov}",
    import_failed: "Could not import the chat",
    inspecting: "Inspecting archive",
    pack_entries: "entries",
    pack_saved: "saved",
    pack_time: "ms",
    inspect_bk: "Inspect",
    bk_research_banner: "Backup research mode · Read only",
    exit_research: "Exit",
    delete_all_backups: "Delete all backups",
    delete_all_bk_title: "Delete all backups",
    delete_all_bk_warn: "All backups across all storages will be permanently deleted:",
    delete_all_confirm: "Delete all",
    deleting_all: "Cleaning",
    bk_count_lbl: "{n} bk.",
    bk_total_lbl: "Total to delete",
    toast_bk_deleted: "Backup deleted · freed {size}",
    toast_all_bk_deleted: "Deleted {n} backups · freed {size}",
    bk_cleanup_done: "Backup cleanup complete",
    bk_deleted_count: "Backups deleted",
    bk_freed_space: "Space freed",
    u_b: "B",
    u_kb: "KB",
    u_mb: "MB",
    u_gb: "GB",
    u_steps: "st.",
    u_files: "f.",
    date_locale: "en-GB",
  },
};

const LANG_KEY = "k1tty_lang";
const PRIVACY_KEY = "k1tty_privacy";

let CUR_LANG = "en";
try {
  const savedLang = localStorage.getItem(LANG_KEY);
  if (savedLang && I18N[savedLang]) CUR_LANG = savedLang;
} catch {}

let PRIVACY_ON = false;
try {
  PRIVACY_ON = localStorage.getItem(PRIVACY_KEY) === "1";
} catch {}

function t(key, vars = null) {
  const dict = I18N[CUR_LANG] || I18N.ru;
  let str = dict[key] ?? I18N.ru[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      str = str.replaceAll("{" + k + "}", String(v));
    }
  }
  return str;
}

function localizeBadge(badge) {
  if (!badge) return "";
  if (CUR_LANG === "ru") return badge;
  return String(badge)
    .replace(/\s*ш\.$/, " " + t("u_steps"))
    .replace(/\s*ф\.$/, " " + t("u_files"));
}

function localizeKind(kind) {
  if (!kind || CUR_LANG === "ru") return kind || t("edit");
  const mapUk = {
    "Вы": "Ви",
    "Пользователь": "Користувач",
    "Ассистент": "Асистент",
    "Прочее": "Інше",
    "Запись": "Запис",
    "Служебная запись": "Службовий запис",
    "Ход прерван": "Хід перервано",
    "вывод": "вивід",
    "reasoning": "Міркування",
    "Сообщение пользователя": "Повідомлення користувача",
    "Ответ Antigravity": "Відповідь Antigravity",
    "Ответ Antigravity CLI": "Відповідь Antigravity CLI",
    "Ответ Codex": "Відповідь Codex",
    "Ответ Claude": "Відповідь Claude",
    "Ответ OpenCode": "Відповідь OpenCode",
    "Системное сообщение": "Системне повідомлення",
    "Служебное сообщение": "Службове повідомлення",
    "Компакция": "Компакція",
    "Компакция (сводка)": "Компакція (зведення)",
    "Сообщение об ошибке": "Повідомлення про помилку",
    "Ошибка OpenCode": "Помилка OpenCode",
    "Вывод инструмента": "Вивід інструменту",
    "Размышления": "Міркування",
    "Инструмент": "Інструмент",
    "Система": "Система",
  };
  const mapEn = {
    "Вы": "You",
    "Пользователь": "User",
    "Ассистент": "Assistant",
    "Прочее": "Other",
    "Запись": "Entry",
    "Служебная запись": "Service entry",
    "Ход прерван": "Turn aborted",
    "вывод": "output",
    "reasoning": "Reasoning",
    "Сообщение пользователя": "User message",
    "Ответ Antigravity": "Antigravity reply",
    "Ответ Antigravity CLI": "Antigravity CLI reply",
    "Ответ Codex": "Codex reply",
    "Ответ Claude": "Claude reply",
    "Ответ OpenCode": "OpenCode reply",
    "Системное сообщение": "System message",
    "Служебное сообщение": "Service message",
    "Компакция": "Compaction",
    "Компакция (сводка)": "Compaction summary",
    "Сообщение об ошибке": "Error message",
    "Ошибка OpenCode": "OpenCode error",
    "Вывод инструмента": "Tool output",
    "Размышления": "Reasoning",
    "Инструмент": "Tool",
    "Система": "System",
  };
  const m = CUR_LANG === "uk" ? mapUk : mapEn;
  if (m[kind]) return m[kind];
  if (kind.startsWith("Инструмент: ")) {
    return t("tool") + ": " + kind.slice("Инструмент: ".length);
  }
  if (kind.startsWith("Вывод инструмента: ")) {
    return (CUR_LANG === "uk" ? "Вивід інструменту: " : "Tool output: ") + kind.slice("Вывод инструмента: ".length);
  }
  return kind;
}

function localizeProviderNote(st) {
  const raw = (st.note || "").replace(/\.$/, "");
  if (CUR_LANG === "ru") return raw;
  const procs = Array.isArray(st.running) && st.running.length
    ? st.running.map((x) => x.split(" (")[0]).filter((v, i, a) => a.indexOf(v) === i).join(", ")
    : "";
  if (!st.available) {
    return CUR_LANG === "uk" ? `Каталог не знайдено: ${st.appdata}` : `Directory not found: ${st.appdata}`;
  }
  if (st.blocked) {
    return CUR_LANG === "uk"
      ? `Файли зайняті (${procs || st.name}). Закрийте застосунок для редагування`
      : `Files locked (${procs || st.name}). Close the app before editing`;
  }
  if (procs) {
    return CUR_LANG === "uk"
      ? `Запущено: ${procs}. Файли доступні, перевідкрийте діалог після змін`
      : `Running: ${procs}. Files available, reopen thread after editing`;
  }
  if (!st.writable) {
    return CUR_LANG === "uk" ? "Доступний лише режим читання" : "Read-only mode";
  }
  return CUR_LANG === "uk" ? "Файли вільні" : "Files ready";
}

function localizeRawLabel(lbl) {
  if (!lbl || CUR_LANG === "ru") return lbl || t("stats_lines");
  if (CUR_LANG === "uk") {
    return String(lbl)
      .replace("строк rollout", "рядків rollout")
      .replace("строк JSONL", "рядків JSONL")
      .replace("шагов в БД", "кроків у БД")
      .replace("сообщений в БД", "повідомлень у БД")
      .replace("строк", "рядків");
  }
  return String(lbl)
    .replace("строк rollout", "rollout lines")
    .replace("строк JSONL", "JSONL lines")
    .replace("шагов в БД", "steps in DB")
    .replace("сообщений в БД", "messages in DB")
    .replace("строк", "lines");
}

function localizeUnlockerPrompt(info) {
  if (CUR_LANG === "ru") return info.prompt || t("unlocker_default");
  const cat = info.category || "";
  if (CUR_LANG === "uk") {
    if (cat.includes("Safety Guard")) return "Цей діалог заблоковано системою безпеки (Safety Guard). Хочете зняти блокування?";
    if (cat.includes("Cyber Policy")) return "Цей діалог заблоковано політикою безпеки (Cyber Policy). Хочете зняти блокування?";
    if (cat.includes("лимит") || (info.code || "").includes("Limit")) return "Досягнуто ліміту запитів у цьому діалозі. Хочете прибрати помилку з історії?";
    return t("unlocker_default");
  }
  if (cat.includes("Safety Guard")) return "This dialogue was blocked by Safety Guard. Do you want to unlock it?";
  if (cat.includes("Cyber Policy")) return "This dialogue was blocked by Cyber Policy. Do you want to unlock it?";
  if (cat.includes("лимит") || (info.code || "").includes("Limit")) return "Usage limit reached in this dialogue. Do you want to clear the error?";
  return t("unlocker_default");
}

function localizeUnlockerCategory(cat) {
  if (!cat || CUR_LANG === "ru") return cat || "";
  if (CUR_LANG === "uk") {
    return String(cat)
      .replace("Блокировка", "Блокування")
      .replace("Ошибка", "Помилка")
      .replace("Ограничение", "Обмеження");
  }
  return String(cat)
    .replace("Блокировка ", "Blocked: ")
    .replace("Ошибка ", "Error: ")
    .replace("Ограничение ", "Limit: ");
}

const PROV = {
  antigravity:     { name: "Antigravity",     icon: "/static/icons/antigravity.svg",     idx: 1 },
  antigravity_cli: { name: "Antigravity CLI", icon: "/static/icons/antigravity-cli.svg", idx: 2 },
  codex:           { name: "Codex",           icon: "/static/icons/codex.svg",           idx: 3 },
  claude:          { name: "Claude",          icon: "/static/icons/claude.svg",          idx: 4 },
  opencode:        { name: "OpenCode",        icon: "/static/icons/opencode.svg",        idx: 5 },
};
const PROV_ORDER = ["antigravity", "antigravity_cli", "codex", "claude", "opencode"];

function aiImg(provId, cls = "") {
  const p = PROV[provId];
  if (!p) return "";
  return `<img${cls ? ` class="${cls}"` : ""} src="${p.icon}" alt="${esc(p.name)}">`;
}

function applyPrivacyUI() {
  document.body.classList.toggle("privacy-mode", PRIVACY_ON);
  const btn = $("#privacyBtn");
  if (btn) {
    btn.classList.toggle("privacy-active", PRIVACY_ON);
    btn.setAttribute("aria-pressed", PRIVACY_ON ? "true" : "false");
    const tip = PRIVACY_ON ? t("privacy_on_tip") : t("privacy_tip");
    btn.title = tip;
    btn.setAttribute("aria-label", tip);
    btn.innerHTML = ico(PRIVACY_ON ? "eye-off" : "eye");
  }
  const titleEl = $("#threadTitle");
  if (titleEl && STATE.active) {
    titleEl.title = PRIVACY_ON ? "" : STATE.active.title;
  }
}

function setPrivacy(next) {
  PRIVACY_ON = !!next;
  try { localStorage.setItem(PRIVACY_KEY, PRIVACY_ON ? "1" : "0"); } catch {}
  applyPrivacyUI();
}

let autoBackupPending = false;
function applyAutoBackupUI() {
  const on = STATE.settings?.auto_backup !== false;
  for (const [inpId, wrapId] of [
    ["#ovAutoBackupToggle", "#ovAutoBackupWrap"],
    ["#bkAutoBackupToggle", "#bkAutoBackupWrap"],
  ]) {
    const inp = $(inpId);
    const wrap = $(wrapId);
    if (inp) {
      inp.checked = on;
      inp.disabled = autoBackupPending;
    }
    if (wrap) {
      wrap.classList.toggle("is-on", on);
      wrap.setAttribute("aria-busy", String(autoBackupPending));
    }
  }
}

async function setAutoBackup(next) {
  if (autoBackupPending) { applyAutoBackupUI(); return; }
  const val = !!next;
  const previous = STATE.settings?.auto_backup !== false;
  if (val === previous) { applyAutoBackupUI(); return; }
  autoBackupPending = true;
  STATE.settings = { ...(STATE.settings || {}), auto_backup: val };
  applyAutoBackupUI();
  try {
    const r = await rpc("set_settings", "all", { auto_backup: val });
    if (r.settings) STATE.settings = r.settings;
    toast(t(val ? "toast_autobk_on" : "toast_autobk_off"), "ok");
  } catch (e) {
    STATE.settings = { ...(STATE.settings || {}), auto_backup: previous };
    toast(e.message, "err");
  } finally {
    autoBackupPending = false;
    applyAutoBackupUI();
  }
}

function applyWinterUI() {
  const on = window.K1ttyWinter ? window.K1ttyWinter.isEnabled() : document.body.classList.contains("winter-theme");
  const btn = $("#winterBtn");
  if (btn) {
    btn.classList.toggle("winter-active", on);
    btn.setAttribute("aria-pressed", on ? "true" : "false");
    const tip = on ? t("winter_on_tip") : t("winter_tip");
    btn.title = tip;
    btn.setAttribute("aria-label", tip);
  }
}

function applyI18n() {
  document.documentElement.lang = CUR_LANG;
  $$("[data-i18n]").forEach((el) => {
    el.textContent = t(el.dataset.i18n);
  });
  $$("[data-i18n-placeholder]").forEach((el) => {
    el.placeholder = t(el.dataset.i18nPlaceholder);
  });
  $$("[data-i18n-title]").forEach((el) => {
    el.title = t(el.dataset.i18nTitle);
  });
  $$("[data-i18n-aria]").forEach((el) => {
    el.setAttribute("aria-label", t(el.dataset.i18nAria));
  });
  const sign = $("#welcomeSign");
  if (sign) {
    sign.innerHTML = `${esc(t("welcome_sign_prefix"))} <span>${esc(t("welcome_sign_names"))}</span>`;
  }
  window.K1ttyDesktop?.setWindowLabels?.({
    minimize: t("window_minimize"),
    maximize: t("window_maximize"),
    restore: t("window_restore"),
    close: t("window_close"),
  });
  applyPrivacyUI();
  applyWinterUI();
  applyAutoBackupUI();
  if (!STATE.active) {
    $("#threadTitle").textContent = t("overview");
    $("#threadTitle").classList.remove("is-thread");
  } else {
    if (!STATE.active.readonlyBackup) {
      updateThreadLockBtn();
    }
    if (STATE.active.blockedInfo && STATE.active.blockedInfo.is_blocked && !$("#unlockerCard")?.classList.contains("hidden")) {
      showUnlockerCard(STATE.active.blockedInfo, false);
    }
    if (STATE.active.lastTurns && !STATE.active.loading && !STATE.active.paging) {
       
       
      STATE.active.languageRender = commitThreadTranscript(STATE.loadingThreadToken, STATE.active.lastTurns, !!STATE.active.hasMore, { preserve: true })
        .catch((error) => toast(error.message, "err"));
    }
  }
  if (noticeCtx) renderNoticeState();
  syncWelcomeControls();
  renderSidebar();
  if (!STATE.active) renderOverview();
  refreshStatusBadge();
}

function setLang(lang) {
  if (!I18N[lang] || CUR_LANG === lang) return Promise.resolve();
  CUR_LANG = lang;
  try { localStorage.setItem(LANG_KEY, lang); } catch {}
  applyI18n();
  return STATE.active?.languageRender || Promise.resolve();
}

let loadCount = 0;
function startLoad() {
  loadCount++;
  $("#topLoader")?.classList.add("active");
}
function stopLoad() {
  loadCount = Math.max(0, loadCount - 1);
  if (loadCount === 0) $("#topLoader")?.classList.remove("active");
}

let WS_READY = false;
let WAS_CONNECTED = false;
const jobHandlers = new Map();
const finishedJobs = new Map();

function onNativeMessage(m) {
  if (m.type === "job") return onJobEvent(m.job);
  if (m.type === "procs") return onProcsUpdate(m.providers);
  if (m.type === "conv_lock") return onConvLockUpdate(m);
  if (m.type === "settings" && m.settings) {
    STATE.settings = m.settings;
    applyAutoBackupUI();
    return;
  }
  if (m.type === "hello") {
    const bid = String(m.boot_id || "");
    try {
      if (bid) sessionStorage.setItem("k1tty_boot_id", bid);
      if (localStorage.getItem("k1tty_welcome_v2_seen") !== "1") {
        requestAnimationFrame(() => openWelcome());
      }
    } catch {
      requestAnimationFrame(() => openWelcome());
    }
  }
}

async function wsConnect() {
  try {
    if (!window.K1ttyDesktop) throw new Error("Start K1tty Tools with the desktop application.");
    await window.K1ttyDesktop.connect(onNativeMessage);
    WS_READY = true;
    WAS_CONNECTED = true;
    if (!window.__booted) {
      window.__booted = true;
      boot();
    } else {
      refreshAll(false);
    }
  } catch (error) {
    WS_READY = false;
    toast(error.message, "err");
  }
}

function rpc(op, provider = "all", data = {}) {
  if (!WS_READY) return Promise.reject(new Error(t("err_no_conn")));
  return window.K1ttyDesktop.rpc(op, provider, data);
}

window.addEventListener("k1tty-error", (event) => toast(event.detail, "err"));

const PHASE_PCT = { start: 12, read: 34, backup: 34, cut: 68, edit: 68, write: 72, lock: 72, unlock: 72, restore: 68, leak: 90, done: 100 };

function onJobEvent(job) {
  const bar = $("#jobBar");
  bar.classList.remove("hidden");
  const pct = PHASE_PCT[job.phase] ?? 20;
  const pkey = "phase_" + job.phase;
  $("#jobPhase").textContent = (I18N[CUR_LANG] && I18N[CUR_LANG][pkey]) ? t(pkey) : job.phase;
  $("#jobPct").textContent = pct + "%";
  $("#jobFill").style.transform = `scaleX(${pct / 100})`;

  if (job.state === "done" || job.state === "error") {
    finishedJobs.set(job.id, job);
    setTimeout(() => {
      bar.classList.add("hidden");
      $("#jobFill").style.transform = "scaleX(0)";
    }, job.state === "done" ? 750 : 1400);
    const h = jobHandlers.get(job.id);
    if (h) {
      jobHandlers.delete(job.id);
      finishedJobs.delete(job.id);
      if (job.state === "done") h.resolve(job);
      else h.reject(new Error(job.error || t("err_op")));
    }
  }
  while (finishedJobs.size > 60) finishedJobs.delete(finishedJobs.keys().next().value);
}

function waitJob(jobId) {
  const done = finishedJobs.get(jobId);
  if (done) {
    finishedJobs.delete(jobId);
    return done.state === "done" ? Promise.resolve(done) : Promise.reject(new Error(done.error));
  }
  return new Promise((resolve, reject) => jobHandlers.set(jobId, { resolve, reject }));
}

const ARMED = new WeakMap();

function disarmButton(btn) {
  const st = ARMED.get(btn);
  if (!st) return;
  clearTimeout(st.timer);
  ARMED.delete(btn);
  btn.classList.remove("armed");
  const lbl = btn.querySelector(".arm-label");
  if (lbl) lbl.textContent = st.origLabel;
  else btn.innerHTML = st.origHtml;
}

function disarmAll(exceptBtn = null) {
  $$(".armed").forEach((b) => {
    if (b !== exceptBtn) disarmButton(b);
  });
}

function handleArmedClick(btn, armedText, onConfirm) {
  if (ARMED.has(btn)) {
    disarmButton(btn);
    onConfirm();
    return;
  }
  disarmAll(btn);
  const lbl = btn.querySelector(".arm-label");
  const origLabel = lbl ? lbl.textContent : "";
  const origHtml = btn.innerHTML;
  if (lbl) {
    lbl.textContent = armedText;
  } else {
    btn.innerHTML = `${ico("warn")}<span>${esc(armedText)}</span><span class="arm-bar"></span>`;
  }
  btn.classList.remove("armed");
  void btn.offsetWidth;
  btn.classList.add("armed");

  const timer = setTimeout(() => disarmButton(btn), 4000);
  ARMED.set(btn, { timer, origLabel, origHtml });
}

document.addEventListener("click", (ev) => {
  if (!ev.target.closest(".armed")) disarmAll();
}, true);

let toastTimer = null;
function toast(msg, cls = "", action = null) {
  const tEl = $("#toast");
  const iconName = cls === "ok" ? "check-circle" : cls === "err" ? "alert" : "info";
  tEl.className = "toast show " + cls;
  tEl.innerHTML = `${ico(iconName)}<span>${esc(msg)}</span>` +
    (action ? `<button class="btn sm" id="toastActBtn">${esc(action.label)}</button>` : "");
  if (action) {
    $("#toastActBtn").onclick = () => {
      tEl.classList.remove("show");
      clearTimeout(toastTimer);
      action.onClick();
    };
  }
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => tEl.classList.remove("show"), action ? 7500 : 3400);
}

function openModal(id) {
  closeCtxMenu();
  disarmAll();
  document.getElementById(id)?.classList.add("open");
}
function closeModal(id) {
  disarmAll();
  document.getElementById(id)?.classList.remove("open");
  if (id === "welcomeModal") {
    stopWelcomeCycle();
    try { localStorage.setItem("k1tty_welcome_v2_seen", "1"); } catch {}
  }
}

function fmtSize(bytes) {
  if (bytes == null || isNaN(bytes)) return "0 " + t("u_b");
  if (bytes < 1024) return Math.round(bytes) + " " + t("u_b");
  if (bytes < 1048576) return Math.round(bytes / 1024) + " " + t("u_kb");
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + " " + t("u_mb");
  return (bytes / 1073741824).toFixed(1) + " " + t("u_gb");
}

const dateFormatters = new Map();
function fmtTime(s) {
  if (!s) return "";
  const d = new Date(s);
  if (isNaN(d)) return "";
  const locale = t("date_locale");
  if (!dateFormatters.has(locale)) dateFormatters.set(locale, new Intl.DateTimeFormat(locale, { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" }));
  return dateFormatters.get(locale).format(d);
}

const CUR_YEAR = String(new Date().getFullYear());
function fmtShortMtime(s) {
  if (!s) return "";
  return String(s).replace("." + CUR_YEAR + " ", " ");
}

const CODE_STORE = [];
const escAttr = (s) => esc(s).replace(/"/g, "&quot;").replace(/'/g, "&#39;");

function isLocalPathTarget(raw) {
  const s = String(raw || "").trim();
  if (!s || /\s{2,}/.test(s) || s.length > 520) return false;
  if (/^file:\/\//i.test(s)) return true;
  if (/^[A-Za-z]:[\\/]/.test(s)) return true;
  if (/^\\\\[^\\/\s]+[\\/][^\\/\s]+/.test(s)) return true;
  if (/^~[\\/]/.test(s)) return true;
  if (/^%(?:USERPROFILE|APPDATA|LOCALAPPDATA|TEMP|TMP)%[\\/]/i.test(s)) return true;
  if (/^\/(?:Users|home|var|etc|opt|tmp|mnt|Volumes|Applications|Library|System|root|srv|usr|proc|dev)\//.test(s)) return true;
  return false;
}

function decodePathDisplay(rawHref) {
  let raw = String(rawHref || "").trim();
  let lineLabel = "";
  const hashIdx = raw.indexOf("#");
  if (hashIdx >= 0) {
    const frag = raw.slice(hashIdx + 1).trim();
    raw = raw.slice(0, hashIdx);
    const lm = frag.match(/^L?(\d+(?:-L?\d+)?)$/i);
    if (lm) lineLabel = `L${lm[1].replace(/^L/i, "")}`;
  }
  const colonMatch = !lineLabel ? raw.match(/^(.*[^:]):(\d+(?:[-:]\d+)?)$/) : null;
  if (colonMatch && colonMatch[1].length > 2) {
    raw = colonMatch[1];
    lineLabel = `:${colonMatch[2]}`;
  }
  let decoded = raw;
  try { decoded = decodeURIComponent(raw); } catch {}
  if (/^file:\/\/\//i.test(decoded)) {
    decoded = decoded.slice(8);
    if (!/^[A-Za-z]:[\\/]/.test(decoded)) decoded = "/" + decoded;
  } else if (/^file:\/\//i.test(decoded)) {
    const rest = decoded.slice(7);
    decoded = /^[A-Za-z]:[\\/]/.test(rest) ? rest : `\\\\${rest.replace(/\//g, "\\")}`;
  }
  if (/^\/[A-Za-z]:[\\/]/.test(decoded)) decoded = decoded.slice(1);
  const isWin = /^[A-Za-z]:[\\/]/.test(decoded) || /^\\\\/.test(decoded) || /^%[A-Za-z]+%[\\/]/.test(decoded);
  const displayPath = isWin ? decoded.replace(/\//g, "\\") : decoded;
  const trimmedEnd = displayPath.replace(/[\\/]+$/, "");
  const parts = trimmedEnd.split(/[\\/]/).filter(Boolean);
  const basename = parts.length ? parts[parts.length - 1] : displayPath;
  const looksDir = /[\\/]$/.test(displayPath) || (parts.length > 0 && !/\.[A-Za-z0-9]{1,12}$/.test(basename));
  return { displayPath, basename, lineLabel, looksDir };
}

function renderFileLinkHTML(label, rawHref, { isInlineCode = false } = {}) {
  const info = decodePathDisplay(rawHref);
  const tip = `${t("reveal_in_explorer")}: ${info.displayPath}`;
  const iconName = info.looksDir ? "folder" : "file-text";
  if (isInlineCode) {
    return `<a class="md-path-code" href="#reveal" data-reveal-path="${escAttr(rawHref)}" title="${escAttr(tip)}">${ico(iconName, "md-path-ico")}<span>${esc(label)}</span></a>`;
  }
  const rawLabel = String(label || "").trim();
  const shown = rawLabel && !/^file:\/\//i.test(rawLabel) ? rawLabel : (info.basename || info.displayPath);
  const showLine = info.lineLabel && !shown.includes(info.lineLabel.replace(/^[:L]/, ""));
  const lineHtml = showLine ? `<span class="md-file-line">${esc(info.lineLabel)}</span>` : "";
  return `<a class="md-file-link" href="#reveal" data-reveal-path="${escAttr(rawHref)}" title="${escAttr(tip)}">${ico(iconName, "md-file-ico")}<span class="md-file-name">${esc(shown)}</span>${lineHtml}</a>`;
}

function renderWebLinkHTML(label, url) {
  if (!/^(?:https?:\/\/|mailto:)/i.test(url)) return esc(label || url);
  const shown = label || url;
  return `<a class="md-link" href="${escAttr(url)}" target="_blank" rel="noopener noreferrer" title="${escAttr(url)}"><span>${esc(shown)}</span>${ico("external", "md-link-ext")}</a>`;
}

function trimTrailingLinkPunct(url) {
  let clean = url;
  let tail = "";
  while (clean.length > 1) {
    const ch = clean[clean.length - 1];
    if (/[.,;:!?]/.test(ch)) {
      clean = clean.slice(0, -1);
      tail = ch + tail;
      continue;
    }
    if (ch === ")" && (clean.split("(").length - 1) < (clean.split(")").length - 1)) {
      clean = clean.slice(0, -1);
      tail = ch + tail;
      continue;
    }
    if (ch === "]" && (clean.split("[").length - 1) < (clean.split("]").length - 1)) {
      clean = clean.slice(0, -1);
      tail = ch + tail;
      continue;
    }
    break;
  }
  return [clean, tail];
}

const MATH_CACHE = new Map();
function renderMathHTML(rawTex, display) {
  const tex = String(rawTex || "").trim();
  if (!tex) return "";
  const cacheKey = (display ? "D:" : "I:") + tex;
  let inner = MATH_CACHE.get(cacheKey);
  if (inner === undefined) {
    inner = null;
    if (window.katex?.renderToString) {
      try {
        inner = window.katex.renderToString(tex, {
          displayMode: !!display,
          throwOnError: false,
          strict: "ignore",
          trust: false,
          output: "htmlAndMathml",
          maxSize: 40,
          maxExpand: 2000,
        });
      } catch {
        inner = null;
      }
    }
    if (MATH_CACHE.size > 1500) MATH_CACHE.clear();
    MATH_CACHE.set(cacheKey, inner);
  }
  const cls = display ? "md-math is-display" : "md-math";
  if (inner == null) {
    const shown = display ? `$$${tex}$$` : `$${tex}$`;
    return `<span class="${cls} is-raw" data-tex="${escAttr(tex)}"${display ? ' data-display="1"' : ""}><code class="inline">${esc(shown)}</code></span>`;
  }
  return `<span class="${cls}" data-tex="${escAttr(tex)}"${display ? ' data-display="1"' : ""}>${inner}</span>`;
}

function looksLikeInlineMath(body) {
  if (!body || /^\s|\s$/.test(body)) return false;
  if (/^\d+(?:[.,]\d+)?$/.test(body)) return false;
  if (/^\d[\d.,]*\s*(?:[kKmMbB]|USD|usd|руб|грн)?$/.test(body)) return false;
  return true;
}

function fmtInline(rawLine) {
  if (!rawLine) return "";
  const inlines = [];
  const stash = (html) => {
    const id = inlines.length;
    inlines.push(html);
    return `\x00IN${id}\x00`;
  };
  let s = String(rawLine);

  s = s.replace(/``([^`\n]+?)``|`([^`\n]+?)`/g, (_, dbl, sgl) => {
    const codeText = dbl !== undefined ? dbl.trim() : sgl;
    if (isLocalPathTarget(codeText)) {
      return stash(renderFileLinkHTML(codeText, codeText, { isInlineCode: true }));
    }
    return stash(`<code class="inline">${esc(codeText)}</code>`);
  });

  s = s.replace(/\\\$/g, () => stash("$"));
  s = s.replace(/\$\$([^\n]+?)\$\$/g, (full, tex) => (tex.trim() ? stash(renderMathHTML(tex, true)) : full));
  s = s.replace(/\\\[([^\n]+?)\\\]/g, (full, tex) => (tex.trim() ? stash(renderMathHTML(tex, true)) : full));
  s = s.replace(/\\\(([^\n]+?)\\\)/g, (full, tex) => (tex.trim() ? stash(renderMathHTML(tex, false)) : full));
  s = s.replace(/(^|[^\\$\w])\$([^$\n]+?)\$(?![\w$])/g, (full, lead, tex) => {
    if (!looksLikeInlineMath(tex)) return full;
    return lead + stash(renderMathHTML(tex, false));
  });

  s = s.replace(
    /\[([^\]\n]+)\]\((?:<([^>\n]+)>|((?:file:\/\/\/?|[A-Za-z]:[\\/]|~[\\/]|%(?:USERPROFILE|APPDATA|LOCALAPPDATA|TEMP|TMP)%[\\/])[^)\n]+)|((?:[^()\s]|\([^()\s]*\))+))\)/g,
    (full, label, angleTarget, pathTarget, stdTarget) => {
      const target = (angleTarget || pathTarget || stdTarget || "").trim();
      if (!target) return full;
      if (isLocalPathTarget(target)) return stash(renderFileLinkHTML(label, target));
      if (/^(?:https?:\/\/|mailto:)/i.test(target)) return stash(renderWebLinkHTML(label, target));
      if (/^[.\w/-]+\.[A-Za-z0-9]{1,12}(?:(?:#L?\d+(?:-L?\d+)?)|(?::\d+(?:[-:]\d+)?))?$/.test(target)) {
        return stash(`<span class="md-file-ref" title="${escAttr(target)}">${ico("file-text", "md-file-ico")}<span class="md-file-name">${esc(label)}</span></span>`);
      }
      return full;
    }
  );

  s = s.replace(/\bfile:\/\/\/?[^\s<>()\[\]"'`,;]+/gi, (match) => {
    const [clean, tail] = trimTrailingLinkPunct(match);
    return stash(renderFileLinkHTML("", clean)) + tail;
  });

  s = s.replace(/\bhttps?:\/\/[^\s<>()\[\]"'`]+/gi, (match) => {
    const [clean, tail] = trimTrailingLinkPunct(match);
    return stash(renderWebLinkHTML(clean, clean)) + tail;
  });

  s = s.replace(/(^|[\s([{"'])([A-Za-z]:\\[^\s<>()\[\]"'`,;*?|]+|~[\\/][^\s<>()\[\]"'`,;*?|]+|%(?:USERPROFILE|APPDATA|LOCALAPPDATA|TEMP|TMP)%\\[^\s<>()\[\]"'`,;*?|]+)/gi, (full, lead, pathMatch) => {
    const [clean, tail] = trimTrailingLinkPunct(pathMatch);
    if (!isLocalPathTarget(clean)) return full;
    return lead + stash(renderFileLinkHTML(clean, clean)) + tail;
  });

  s = esc(s);
  s = s.replace(/\*\*\*([^\s*](?:[^*\n]*[^\s*])?)\*\*\*/g, "<strong><em>$1</em></strong>");
  s = s.replace(/\*\*([^\s*](?:[^*\n]*[^\s*])?)\*\*/g, "<strong>$1</strong>");
  s = s.replace(/(^|[^\w])__([^\s_](?:[^_\n]*[^\s_])?)__(?=[^\w]|$)/g, "$1<strong>$2</strong>");
  s = s.replace(/~~([^\s~](?:[^~\n]*[^\s~])?)~~/g, "<del>$1</del>");
  s = s.replace(/(^|[^*\w])\*([^\s*](?:[^*\n]*[^\s*])?)\*(?![*\w])/g, "$1<em>$2</em>");
  s = s.replace(/(^|[^\w])_([^\s_](?:[^_\n]*[^\s_])?)_(?=[^\w]|$)/g, "$1<em>$2</em>");

  return s.replace(/\x00IN(\d+)\x00/g, (tok, idx) => inlines[Number(idx)] ?? tok);
}

function splitTableRow(line) {
  let s = line.trim();
  if (s.startsWith("|")) s = s.slice(1);
  if (s.endsWith("|") && !s.endsWith("\\|")) s = s.slice(0, -1);
  const cells = [];
  let cur = "";
  let inCode = false;
  let inMath = false;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (ch === "\\" && i + 1 < s.length && s[i + 1] === "|") {
      cur += "|";
      i++;
      continue;
    }
    if (ch === "`" && !inMath) {
      inCode = !inCode;
      cur += ch;
      continue;
    }
    if (ch === "$" && !inCode && s[i - 1] !== "\\") {
      if (inMath) {
        inMath = false;
      } else {
        const next = s[i + 1] || "";
        const closeAt = s.indexOf("$", i + 1);
        if (closeAt > i + 1 && next && !/[\s\d$]/.test(next) && s.slice(i + 1, closeAt).includes("|")) inMath = true;
      }
      cur += ch;
      continue;
    }
    if (ch === "|" && !inCode && !inMath) {
      cells.push(cur.trim());
      cur = "";
      continue;
    }
    cur += ch;
  }
  cells.push(cur.trim());
  return cells;
}

function parseTableSeparator(line) {
  if (!line || !line.includes("|") && !/^\s*:?-{2,}:?\s*$/.test(line)) return null;
  const cells = splitTableRow(line);
  if (!cells.length) return null;
  const aligns = [];
  for (const c of cells) {
    if (!/^:?-+:?$/.test(c)) return null;
    if (c.startsWith(":") && c.endsWith(":")) aligns.push("center");
    else if (c.endsWith(":")) aligns.push("right");
    else if (c.startsWith(":")) aligns.push("left");
    else aligns.push("");
  }
  return aligns;
}

function renderCodeBlockHTML(lang, rawCode) {
  const idx = CODE_STORE.length;
  const cleanCode = rawCode.replace(/\n$/, "");
  CODE_STORE.push(cleanCode);
  const lowerLang = (lang || "").toLowerCase();
  let innerHtml = esc(cleanCode);
  if (lowerLang === "diff" || lowerLang === "patch") {
    innerHtml = cleanCode.split("\n").map((ln) => {
      if (ln.startsWith("+") && !ln.startsWith("+++")) return `<span class="cd-add">${esc(ln)}</span>`;
      if (ln.startsWith("-") && !ln.startsWith("---")) return `<span class="cd-del">${esc(ln)}</span>`;
      if (ln.startsWith("@@")) return `<span class="cd-hunk">${esc(ln)}</span>`;
      return esc(ln);
    }).join("\n");
  }
  return `<div class="code-wrap"><div class="code-head"><span>${esc(lang || "code")}</span><button class="btn sm" data-act="copy-code" data-code-idx="${idx}">${ico("copy")}<span>${esc(t("copy"))}</span></button></div><pre class="codeblock">${innerHtml}</pre></div>`;
}

function fmtMarkdown(raw) {
  if (!raw) return `<span class="empty">${esc(t("empty"))}</span>`;
  const blocks = [];
  const pushBlock = (html) => {
    const id = blocks.length;
    blocks.push(html);
    return `\x00BLK${id}\x00`;
  };

  const normalized = String(raw).replace(/\x00/g, "").replace(/\r\n?/g, "\n");
  const rawLines = normalized.split("\n");
  const preLines = [];
  let li = 0;
  while (li < rawLines.length) {
    const curLine = rawLines[li];
    const trimmedLine = curLine.trim();
    const oneLineDisplay = trimmedLine.match(/^\$\$((?:(?!\$\$)[\s\S])+)\$\$$/) || trimmedLine.match(/^\\\[((?:(?!\\\])[\s\S])+)\\\]$/);
    if (oneLineDisplay && oneLineDisplay[1].trim()) {
      preLines.push(pushBlock(renderMathHTML(oneLineDisplay[1], true)));
      li++;
      continue;
    }
    const dollarOpen = /^\$\$/.test(trimmedLine) && !/^\$\$[\s\S]*\$\$/.test(trimmedLine);
    const bracketOpen = /^\\\[/.test(trimmedLine) && !/\\\]/.test(trimmedLine);
    if (dollarOpen || bracketOpen) {
      const closer = dollarOpen ? "$$" : "\\]";
      const texLines = [trimmedLine.slice(2)];
      let lj = li + 1;
      let closed = false;
      while (lj < rawLines.length && lj - li <= 80) {
        const ln = rawLines[lj];
        const pos = ln.indexOf(closer);
        if (pos >= 0) {
          texLines.push(ln.slice(0, pos));
          closed = !ln.slice(pos + closer.length).trim();
          break;
        }
        texLines.push(ln);
        lj++;
      }
      if (closed && lj < rawLines.length) {
        const tex = texLines.join("\n");
        if (tex.trim()) {
          preLines.push(pushBlock(renderMathHTML(tex, true)));
          li = lj + 1;
          continue;
        }
      }
    }
    const fenceOpen = curLine.match(/^ {0,3}(`{3,}|~{3,})\s*([a-zA-Z0-9_+#.-]*)\s*$/);
    if (fenceOpen) {
      const marker = fenceOpen[1];
      const ch = marker[0];
      const len = marker.length;
      const lang = fenceOpen[2] || "";
      const codeLines = [];
      li++;
      while (li < rawLines.length) {
        const closeRe = new RegExp(`^ {0,3}${ch}{${len},}\\s*$`);
        if (closeRe.test(rawLines[li])) {
          li++;
          break;
        }
        codeLines.push(rawLines[li]);
        li++;
      }
      if (/^(?:math|katex)$/i.test(lang) && codeLines.join("").trim()) {
        preLines.push(pushBlock(renderMathHTML(codeLines.join("\n"), true)));
        continue;
      }
      preLines.push(pushBlock(renderCodeBlockHTML(lang, codeLines.join("\n"))));
      continue;
    }
    preLines.push(rawLines[li].replace(/```([a-zA-Z0-9_+-]*)\s*([^`\n]+)```/g, (_, lang, code) => pushBlock(renderCodeBlockHTML(lang, code))));
    li++;
  }

  const outLines = [];
  let i = 0;
  const isBlockTok = (s) => /^\x00BLK\d+\x00$/.test(s);
  const listRe = /^(\s*)(?:([-*+])|(\d+)[.)])\s+(.*)$/;

  while (i < preLines.length) {
    const line = preLines[i];
    if (isBlockTok(line)) {
      outLines.push(line);
      i++;
      continue;
    }

    if (line.includes("|") && i + 1 < preLines.length) {
      const aligns = parseTableSeparator(preLines[i + 1]);
      if (aligns) {
        const headerCells = splitTableRow(line);
        if (headerCells.length === aligns.length || (headerCells.length >= 2 && aligns.length >= 2)) {
          const cols = Math.max(headerCells.length, aligns.length);
          const thHtml = Array.from({ length: cols }, (_, c) => {
            const al = aligns[c] ? ` style="text-align:${aligns[c]}"` : "";
            return `<th${al}>${fmtInline(headerCells[c] || "")}</th>`;
          }).join("");
          i += 2;
          const bodyRows = [];
          while (i < preLines.length) {
            const rowLine = preLines[i];
            if (!rowLine.trim() || !rowLine.includes("|") || isBlockTok(rowLine) || /^\s*#{1,6}\s/.test(rowLine) || /^\s*(?:---+|\*\*\*+|___+)\s*$/.test(rowLine)) break;
            const cells = splitTableRow(rowLine);
            const tdHtml = Array.from({ length: cols }, (_, c) => {
              const al = aligns[c] ? ` style="text-align:${aligns[c]}"` : "";
              return `<td${al}>${fmtInline(cells[c] || "")}</td>`;
            }).join("");
            bodyRows.push(`<tr>${tdHtml}</tr>`);
            i++;
          }
          outLines.push(pushBlock(`<div class="md-table-wrap"><table class="md-table"><thead><tr>${thHtml}</tr></thead><tbody>${bodyRows.join("")}</tbody></table></div>`));
          continue;
        }
      }
    }

    if (/^\s*(?:---+|\*\*\*+|___+)\s*$/.test(line)) {
      outLines.push(pushBlock('<hr class="md-hr">'));
      i++;
      continue;
    }

    const hm = line.match(/^\s*(#{1,6})\s+(.+?)\s*#*$/);
    if (hm) {
      const lvl = Math.min(hm[1].length, 4);
      outLines.push(pushBlock(`<h${lvl} class="md-h md-h${lvl}">${fmtInline(hm[2])}</h${lvl}>`));
      i++;
      continue;
    }

    if (/^\s*>\s?/.test(line)) {
      const qLines = [];
      while (i < preLines.length && /^\s*>\s?/.test(preLines[i])) {
        qLines.push(preLines[i].replace(/^\s*>\s?/, ""));
        i++;
      }
      const alertMatch = qLines[0]?.match(/^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*(.*)$/i);
      if (alertMatch) {
        const kind = alertMatch[1].toLowerCase();
        const iconMap = { note: "info", tip: "sparkles", important: "alert", warning: "warn", caution: "alert" };
        const restLines = alertMatch[2] ? [alertMatch[2], ...qLines.slice(1)] : qLines.slice(1);
        const bodyHtml = restLines.map((l) => fmtInline(l)).join("<br>");
        outLines.push(pushBlock(`<blockquote class="md-quote md-callout is-${kind}"><div class="md-callout-head">${ico(iconMap[kind] || "info")}<span>${esc(alertMatch[1].toUpperCase())}</span></div><div class="md-callout-body">${bodyHtml}</div></blockquote>`));
      } else {
        const bodyHtml = qLines.map((l) => fmtInline(l)).join("<br>");
        outLines.push(pushBlock(`<blockquote class="md-quote">${bodyHtml}</blockquote>`));
      }
      continue;
    }

    const lm = line.match(listRe);
    if (lm) {
      const isOrdered = !!lm[3];
      const baseIndent = lm[1].length;
      const items = [];
      while (i < preLines.length) {
        const cur = preLines[i];
        const cm = cur.match(listRe);
        if (!cm) break;
        const curIndent = cm[1].length;
        const curOrdered = !!cm[3];
        if (curIndent < baseIndent) break;
        if (curIndent === baseIndent && curOrdered !== isOrdered) break;
        const textPart = cm[4];
        const taskMatch = textPart.match(/^\[([ xX])\]\s+(.*)$/);
        let rendered;
        if (taskMatch) {
          const done = taskMatch[1].toLowerCase() === "x";
          rendered = `<span class="md-task${done ? " is-done" : ""}">${done ? ico("check") : ""}</span>${fmtInline(taskMatch[2])}`;
        } else {
          rendered = fmtInline(textPart);
        }
        if (curIndent > baseIndent && items.length) {
          items[items.length - 1].sub.push({ ordered: curOrdered, html: rendered });
        } else {
          items.push({ html: rendered, sub: [] });
        }
        i++;
      }
      const tag = isOrdered ? "ol" : "ul";
      const cls = isOrdered ? "md-list md-olist" : "md-list";
      const liHtml = items.map((it) => {
        if (!it.sub.length) return `<li>${it.html}</li>`;
        const subTag = it.sub[0].ordered ? "ol" : "ul";
        const subCls = it.sub[0].ordered ? "md-list md-olist" : "md-list";
        return `<li>${it.html}<${subTag} class="${subCls}">${it.sub.map((s) => `<li>${s.html}</li>`).join("")}</${subTag}></li>`;
      }).join("");
      outLines.push(pushBlock(`<${tag} class="${cls}">${liHtml}</${tag}>`));
      continue;
    }

    outLines.push(fmtInline(line));
    i++;
  }

  let joined = outLines.join("\n");
  joined = joined.replace(/\n{0,2}(\x00BLK\d+\x00)\n{0,2}/g, "$1");
  return joined.replace(/\x00BLK(\d+)\x00/g, (tok, idx) => blocks[Number(idx)] ?? tok);
}

function fmtToolText(raw) {
  if (!raw) return `<span class="empty">${esc(t("empty"))}</span>`;
  const tokens = [];
  const stash = (html) => {
    const id = tokens.length;
    tokens.push(html);
    return `\x00TP${id}\x00`;
  };
  let s = String(raw).replace(/\x00/g, "");

  s = s.replace(/\bfile:\/\/\/?[^\s<>()\[\]"'`,;]+/gi, (match) => {
    const [clean, tail] = trimTrailingLinkPunct(match);
    const info = decodePathDisplay(clean);
    const tip = `${t("reveal_in_explorer")}: ${info.displayPath}`;
    return stash(`<a class="md-tool-path" href="#reveal" data-reveal-path="${escAttr(clean)}" title="${escAttr(tip)}">${esc(clean)}</a>`) + tail;
  });

  s = s.replace(/"([A-Za-z]:(?:\\\\|\\|\/)[^"\r\n]+)"/g, (full, inner) => {
    const unescaped = inner.replace(/\\\\/g, "\\");
    if (!isLocalPathTarget(unescaped)) return full;
    const info = decodePathDisplay(unescaped);
    const tip = `${t("reveal_in_explorer")}: ${info.displayPath}`;
    return `"${stash(`<a class="md-tool-path" href="#reveal" data-reveal-path="${escAttr(unescaped)}" title="${escAttr(tip)}">${esc(inner)}</a>`)}"`;
  });

  s = s.replace(/(^|[\s([{'=:])([A-Za-z]:\\[^\s<>()\[\]"'`,;*?|]+|~[\\/][^\s<>()\[\]"'`,;*?|]+)/g, (full, lead, pathMatch) => {
    const [clean, tail] = trimTrailingLinkPunct(pathMatch);
    if (!isLocalPathTarget(clean)) return full;
    const info = decodePathDisplay(clean);
    const tip = `${t("reveal_in_explorer")}: ${info.displayPath}`;
    return lead + stash(`<a class="md-tool-path" href="#reveal" data-reveal-path="${escAttr(clean)}" title="${escAttr(tip)}">${esc(clean)}</a>`) + tail;
  });

  s = esc(s);
  return s.replace(/\x00TP(\d+)\x00/g, (tok, idx) => tokens[Number(idx)] ?? tok);
}

async function revealLocalPath(rawPath) {
  if (!rawPath) return;
  if (!window.K1ttyDesktop?.revealPath) {
    toast(t("reveal_desktop_only"), "err");
    return;
  }
  try {
    await window.K1ttyDesktop.revealPath(rawPath);
  } catch (e) {
    toast(e?.message || String(e), "err");
  }
}

function convListSkeleton() {
  let html = "";
  for (let i = 0; i < 8; i++) {
    html += `<div class="skel-item"><div class="skel-bar skel-w90"></div><div class="skel-bar skel-w50"></div></div>`;
  }
  return html;
}

function feedSkeleton() {
  let html = "";
  for (let i = 0; i < 4; i++) {
    html += `<div class="skel-msg"><div class="skel-bar skel-w30"></div><div class="skel-bar skel-w90"></div><div class="skel-bar skel-w75"></div></div>`;
  }
  return html;
}

const SELECT_SCOPE = ".codeblock, .md-math.is-display, .md-table-wrap, .tool-body, .msg-body, .report-body, .preview-text, .diff-box, .unlocker-body, .rep-card";
const COPY_CHROME = "button, .code-head, .msg-actions, .clamp-bar, .md-file-ico, .md-path-ico, .md-link-ext, .prov-path-ico, .md-callout-head svg, svg.ico";
let lastPointerTarget = null;

function selectionTouches(el) {
  const sel = window.getSelection?.();
  if (!sel || sel.isCollapsed || !sel.toString()) return false;
  try {
    return sel.containsNode(el, true);
  } catch {
    return false;
  }
}

function selectNodeContents(el) {
  if (!el) return false;
  const range = document.createRange();
  range.selectNodeContents(el);
  const sel = window.getSelection();
  sel.removeAllRanges();
  sel.addRange(range);
  return true;
}

function mathSource(mathEl) {
  const tex = mathEl.dataset.tex || "";
  return mathEl.dataset.display === "1" ? `\n$$${tex}$$\n` : `$${tex}$`;
}

function tidyCopyClone(root) {
  root.querySelectorAll(".md-math").forEach((m) => m.replaceWith(document.createTextNode(mathSource(m))));
  root.querySelectorAll(".katex").forEach((k) => {
    const ann = k.querySelector('annotation[encoding="application/x-tex"]');
    k.replaceWith(document.createTextNode(ann ? `$${ann.textContent}$` : k.textContent || ""));
  });
  root.querySelectorAll(COPY_CHROME).forEach((n) => n.remove());
  root.querySelectorAll(".md-task").forEach((n) => n.replaceWith(document.createTextNode(n.classList.contains("is-done") ? "[x] " : "[ ] ")));
}

function renderedText(root) {
  const host = document.createElement("div");
  host.className = "copy-host";
  host.style.cssText = "position:fixed;left:-100000px;top:0;width:900px;white-space:pre-wrap;opacity:0;pointer-events:none;";
  host.appendChild(root);
  document.body.appendChild(host);
  const text = host.innerText;
  host.remove();
  return text.replace(/\u00a0/g, " ").replace(/\n{3,}/g, "\n\n").replace(/^\n+|\n+$/g, "");
}

function serializeSelection() {
  const sel = window.getSelection?.();
  if (!sel || sel.isCollapsed || !sel.rangeCount) return null;
  const wrap = document.createElement("div");
  let needsCustom = false;
  for (let i = 0; i < sel.rangeCount; i++) {
    const range = sel.getRangeAt(i).cloneRange();
    const startEl = range.startContainer.nodeType === 1 ? range.startContainer : range.startContainer.parentElement;
    const endEl = range.endContainer.nodeType === 1 ? range.endContainer : range.endContainer.parentElement;
    const startMath = startEl?.closest?.(".md-math");
    const endMath = endEl?.closest?.(".md-math");
    if (startMath) range.setStartBefore(startMath);
    if (endMath) range.setEndAfter(endMath);
    const frag = range.cloneContents();
    if (frag.querySelector?.(".md-math, .katex, button, .code-head, .msg-actions, .md-task, svg")) needsCustom = true;
    if (startMath || endMath) needsCustom = true;
    if (i > 0) wrap.appendChild(document.createTextNode("\n"));
    wrap.appendChild(frag);
  }
  if (!needsCustom) return { text: sel.toString(), html: "", custom: false };
  tidyCopyClone(wrap);
  const html = wrap.innerHTML;
  return { text: renderedText(wrap), html, custom: true };
}

function serializeNode(node) {
  const clone = node.cloneNode(true);
  const holder = document.createElement("div");
  holder.appendChild(clone);
  tidyCopyClone(holder);
  return renderedText(holder);
}

function tableToMarkdown(table) {
  const rows = [...table.querySelectorAll("tr")].map((tr) => [...tr.children].map((cell) => serializeNode(cell).replace(/\n+/g, " ").replace(/\|/g, "\\|").trim()));
  if (!rows.length) return "";
  const cols = Math.max(...rows.map((r) => r.length));
  const line = (r) => `| ${Array.from({ length: cols }, (_, i) => r[i] || "").join(" | ")} |`;
  const out = [line(rows[0]), `| ${Array.from({ length: cols }, () => "---").join(" | ")} |`];
  for (const r of rows.slice(1)) out.push(line(r));
  return out.join("\n");
}

async function copyPlainText(text) {
  if (!text) return;
  try {
    if (window.K1ttyDesktop?.writeText) await window.K1ttyDesktop.writeText(text);
    else await navigator.clipboard.writeText(text);
    toast(t("tm_copied"), "ok");
  } catch (e) {
    toast(e?.message || String(e), "err");
  }
}

function isEditableTarget(el) {
  return !!el?.closest?.('input:not([type="button"]):not([type="checkbox"]):not([type="radio"]):not([type="range"]):not([type="color"]), textarea');
}

document.addEventListener("copy", (ev) => {
  if (isEditableTarget(document.activeElement)) return;
  const data = serializeSelection();
  if (!data || !data.custom || !ev.clipboardData) return;
  ev.preventDefault();
  ev.clipboardData.setData("text/plain", data.text);
  if (data.html) ev.clipboardData.setData("text/html", data.html);
});

document.addEventListener("pointermove", (ev) => { lastPointerTarget = ev.target; }, { passive: true, capture: true });

window.addEventListener("k1tty:selectall", () => {
  const sel = window.getSelection();
  const anchor = sel?.anchorNode ? (sel.anchorNode.nodeType === 1 ? sel.anchorNode : sel.anchorNode.parentElement) : null;
  const fromSel = !sel?.isCollapsed ? anchor?.closest?.(SELECT_SCOPE) : null;
  const fromPointer = lastPointerTarget?.closest?.(SELECT_SCOPE);
  const scope = fromSel || fromPointer;
  if (scope) {
    const already = !sel.isCollapsed && sel.toString().trim() === (scope.innerText || "").trim();
    if (!already) { selectNodeContents(scope); return; }
  }
  const modal = document.querySelector(".modal-bg.open .modal-body, .modal-bg.open .modal");
  if (modal) { selectNodeContents(modal); return; }
  const chat = $("#chatWrap");
  if (chat && !chat.classList.contains("hidden")) selectNodeContents($("#chatFeed"));
});

let textMenuActions = [];

function closeTextMenu() {
  const menu = $("#textMenu");
  if (!menu || menu.classList.contains("hidden")) return;
  menu.classList.add("hidden");
  menu.innerHTML = "";
  textMenuActions = [];
}

function buildTextMenu(target) {
  const groups = [];
  const editable = isEditableTarget(target) ? target.closest("input, textarea") : null;
  if (editable) {
    const start = editable.selectionStart ?? 0;
    const end = editable.selectionEnd ?? 0;
    const picked = end > start ? editable.value.slice(start, end) : "";
    const g = [];
    if (picked && !editable.readOnly && !editable.disabled) {
      g.push({ icon: "scissors-cut", label: t("tm_cut"), run: async () => {
        await copyPlainText(picked);
        editable.focus();
        editable.setRangeText("", start, end, "end");
        editable.dispatchEvent(new Event("input", { bubbles: true }));
      } });
    }
    if (picked) g.push({ icon: "copy", label: t("tm_copy"), run: () => copyPlainText(picked) });
    if (editable.value) g.push({ icon: "text-select", label: t("tm_select_all"), run: () => { editable.focus(); editable.select(); } });
    if (g.length) groups.push(g);
    return groups;
  }

  const selData = serializeSelection();
  if (selData?.text?.trim()) groups.push([{ icon: "copy", label: t("tm_copy"), run: () => copyPlainText(selData.text) }]);

  const extra = [];
  const math = target.closest(".md-math");
  if (math?.dataset.tex) extra.push({ icon: "sigma", label: t("tm_copy_tex"), run: () => copyPlainText(math.dataset.tex) });
  const code = target.closest(".code-wrap");
  if (code) extra.push({ icon: "file-text", label: t("tm_copy_code"), run: () => copyPlainText(code.querySelector("pre")?.textContent || "") });
  const cell = target.closest(".md-table td, .md-table th");
  if (cell) {
    extra.push({ icon: "copy", label: t("tm_copy_cell"), run: () => copyPlainText(serializeNode(cell).trim()) });
    const table = cell.closest("table");
    extra.push({ icon: "layers", label: t("tm_copy_table"), run: () => copyPlainText(tableToMarkdown(table)) });
  }
  const link = target.closest("a.md-link[href]");
  if (link) extra.push({ icon: "link", label: t("tm_copy_link"), run: () => copyPlainText(link.getAttribute("href")) });
  const reveal = target.closest("[data-reveal-path]");
  if (reveal) {
    const raw = reveal.dataset.revealPath;
    extra.push({ icon: "folder", label: t("reveal_in_explorer"), run: () => revealLocalPath(raw) });
    extra.push({ icon: "clipboard", label: t("tm_copy_path"), run: () => copyPlainText(decodePathDisplay(raw).displayPath) });
  }
  if (extra.length) groups.push(extra);

  const block = target.closest("#chatFeed [data-key]");
  if (block) {
    const g = [];
    const body = block.querySelector(".msg-body, .tool-body");
    if (body) g.push({ icon: "text-select", label: t("tm_select_msg"), run: () => {
      const details = block.querySelector("details");
      if (details && !details.open) details.open = true;
      selectNodeContents(body);
    } });
    const rawText = block.__k1ttyItem?.text;
    if (rawText) g.push({ icon: "clipboard", label: t("tm_copy_msg"), run: () => copyPlainText(rawText) });
    if (g.length) groups.push(g);
  }

  const chat = $("#chatWrap");
  if (target.closest("#chatFeed") && chat && !chat.classList.contains("hidden")) {
    groups.push([{ icon: "check-square", label: t("tm_select_all"), run: () => selectNodeContents($("#chatFeed")) }]);
  } else if (target.closest(SELECT_SCOPE + ", .modal-body")) {
    const scope = target.closest(SELECT_SCOPE + ", .modal-body");
    groups.push([{ icon: "check-square", label: t("tm_select_all"), run: () => selectNodeContents(scope) }]);
  }
  return groups;
}

function openTextMenu(x, y, groups) {
  const menu = $("#textMenu");
  if (!menu) return;
  textMenuActions = [];
  const parts = [];
  groups.forEach((g, gi) => {
    if (gi > 0) parts.push('<div class="ctx-sep"></div>');
    for (const item of g) {
      const idx = textMenuActions.push(item) - 1;
      parts.push(`<button type="button" class="ctx-item" role="menuitem" data-tm="${idx}">${ico(item.icon)}<span>${esc(item.label)}</span></button>`);
    }
  });
  menu.innerHTML = parts.join("");
  menu.classList.remove("hidden");
  const mw = menu.offsetWidth || 220;
  const mh = menu.offsetHeight || 160;
  menu.style.left = Math.max(8, Math.min(x, window.innerWidth - mw - 8)) + "px";
  menu.style.top = Math.max(8, Math.min(y, window.innerHeight - mh - 8)) + "px";
}

document.addEventListener("contextmenu", (ev) => {
  if (ev.target.closest("#textMenu, #ctxMenu")) return;
  if (ev.target.closest("#convList .conv-item")) { closeTextMenu(); return; }
  const groups = buildTextMenu(ev.target);
  if (!groups.length) { closeTextMenu(); return; }
  closeCtxMenu();
  openTextMenu(ev.clientX, ev.clientY, groups);
});

$("#textMenu")?.addEventListener("mousedown", (ev) => ev.preventDefault());
$("#textMenu")?.addEventListener("click", (ev) => {
  const btn = ev.target.closest("[data-tm]");
  if (!btn) return;
  ev.stopPropagation();
  const action = textMenuActions[Number(btn.dataset.tm)];
  closeTextMenu();
  action?.run();
});
document.addEventListener("mousedown", (ev) => { if (!ev.target.closest("#textMenu")) closeTextMenu(); }, true);
document.addEventListener("keydown", (ev) => { if (ev.key === "Escape") closeTextMenu(); });
window.addEventListener("blur", closeTextMenu);
window.addEventListener("resize", closeTextMenu);
document.addEventListener("scroll", closeTextMenu, true);

const STATE = {
  providers: {},
  allConvs: [],
  backupsSummary: null,
  settings: { auto_backup: true },
  filterProv: "all",
  searchQuery: "",
  active: null,
  showSys: false,
  cursor: null,
  loadedMsgs: 0,
  loadingThreadToken: 0,
};

function isMobile() { return window.innerWidth <= 900; }

let drawerDesktopOpen = !$("#appRoot").classList.contains("drawer-collapsed");
let drawerMobileOpen = $("#appRoot").classList.contains("drawer-mobile-open");
let drawerMobileMode = isMobile();
let drawerCloseTimer = 0;
let drawerResizeFrame = 0;
let drawerClosingFocus = null;

function drawerMotionReduced() {
  if (window.K1ttyAppearance?.isReducedMotion) return window.K1ttyAppearance.isReducedMotion();
  const mode = document.documentElement.dataset.motion;
  return mode ? mode === "reduced" : !!window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
}

function syncDrawerAccessibility(open) {
  const drawer = $("#drawer");
  drawer.inert = !open;
  drawer.setAttribute("aria-hidden", String(!open));
  $("#drawerOpenBtn").setAttribute("aria-expanded", String(open));
  $("#drawerOpenBtn").setAttribute("aria-controls", "drawer");
  $("#drawerCloseBtn").setAttribute("aria-controls", "drawer");
}

function cancelDrawerClose() {
  clearTimeout(drawerCloseTimer);
  drawerCloseTimer = 0;
  $("#appRoot").classList.remove("drawer-closing");
}

function finishDrawerClose() {
  if (drawerDesktopOpen || drawerMobileMode) return;
  cancelDrawerClose();
  $("#appRoot").classList.add("drawer-collapsed");
  syncDrawerAccessibility(false);
  if (drawerClosingFocus && (document.activeElement === document.body || $("#drawer").contains(document.activeElement))) {
    $("#drawerOpenBtn").focus({ preventScroll: true });
  }
  drawerClosingFocus = null;
}

function syncDrawerViewport() {
  const mobile = isMobile();
  if (mobile === drawerMobileMode) return;
  const focusedInside = $("#drawer").contains(document.activeElement);
  const focusedOpener = document.activeElement === $("#drawerOpenBtn");
  cancelDrawerClose();
  drawerClosingFocus = null;
  drawerMobileMode = mobile;
  drawerMobileOpen = false;
  const root = $("#appRoot");
  root.classList.remove("drawer-mobile-open");
  root.classList.toggle("drawer-collapsed", !drawerDesktopOpen);
  const open = mobile ? drawerMobileOpen : drawerDesktopOpen;
  syncDrawerAccessibility(open);
  if (!open && focusedInside) $("#drawerOpenBtn").focus({ preventScroll: true });
  else if (open && focusedOpener) $("#drawerCloseBtn").focus({ preventScroll: true });
}

function toggleDrawer(force) {
  syncDrawerViewport();
  const root = $("#appRoot");
  if (drawerMobileMode) {
    drawerMobileOpen = force !== undefined ? !!force : !drawerMobileOpen;
    const focusedInside = $("#drawer").contains(document.activeElement);
    root.classList.toggle("drawer-mobile-open", drawerMobileOpen);
    syncDrawerAccessibility(drawerMobileOpen);
    if (!drawerMobileOpen && focusedInside) $("#drawerOpenBtn").focus({ preventScroll: true });
    else if (drawerMobileOpen && !focusedInside) $("#drawerCloseBtn").focus({ preventScroll: true });
    return;
  }
  const open = force !== undefined ? !!force : !drawerDesktopOpen;
  if (open === drawerDesktopOpen) return;
  drawerDesktopOpen = open;
  if (open) {
    const restoreFocus = drawerClosingFocus;
    const focusedOpener = document.activeElement === $("#drawerOpenBtn");
    cancelDrawerClose();
    drawerClosingFocus = null;
    root.classList.remove("drawer-collapsed");
    syncDrawerAccessibility(true);
    if (focusedOpener) $("#drawerCloseBtn").focus({ preventScroll: true });
    else if (restoreFocus?.isConnected && document.activeElement === document.body) restoreFocus.focus({ preventScroll: true });
    return;
  }
  drawerClosingFocus = $("#drawer").contains(document.activeElement) ? document.activeElement : null;
  syncDrawerAccessibility(false);
  if (drawerMotionReduced() || document.visibilityState === "hidden") { finishDrawerClose(); return; }
  root.classList.add("drawer-closing");
  drawerCloseTimer = setTimeout(finishDrawerClose, 200);
}

$(".drawer-inner").addEventListener("transitionend", (event) => {
  if (event.target === $(".drawer-inner") && event.propertyName === "opacity" && $("#appRoot").classList.contains("drawer-closing")) finishDrawerClose();
});
window.addEventListener("resize", () => {
  if (!drawerResizeFrame) drawerResizeFrame = requestAnimationFrame(() => { drawerResizeFrame = 0; syncDrawerViewport(); });
}, { passive: true });
window.addEventListener("k1tty:appearancechange", () => {
  if (drawerMotionReduced() && $("#appRoot").classList.contains("drawer-closing")) finishDrawerClose();
});
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "hidden" && $("#appRoot").classList.contains("drawer-closing")) finishDrawerClose();
});
syncDrawerAccessibility(drawerMobileMode ? drawerMobileOpen : drawerDesktopOpen);

$("#drawerOpenBtn").onclick = () => toggleDrawer(true);
$("#drawerCloseBtn").onclick = () => toggleDrawer(false);
$("#drawerBackdrop").onclick = () => toggleDrawer(false);
$("#brandHome").onclick = () => showOverview();
$("#ovBackupsBtn").onclick = () => openBackups("all", null);
$("#ovBkExploreBtn")?.addEventListener("click", () => openBackups("all", null));
$("#ovBkCleanBtn")?.addEventListener("click", () => openDeleteAllBackups());
$("#ovAutoBackupToggle")?.addEventListener("change", (ev) => setAutoBackup(ev.target.checked));
$("#bkAutoBackupToggle")?.addEventListener("change", (ev) => setAutoBackup(ev.target.checked));
$("#ovStats")?.addEventListener("click", (ev) => {
  if (ev.target.closest("[data-open-backups]")) openBackups("all", null);
});
$("#ovBkGrid")?.addEventListener("click", (ev) => {
  const card = ev.target.closest("[data-bk-prov]");
  if (!card) return;
  openBackups(card.dataset.bkProv, null);
});
$("#ovImportBtn").onclick = () => openImportModal();
$("#importQuickBtn").onclick = () => openImportModal();
$("#winterBtn")?.addEventListener("click", () => {
  if (!window.K1ttyWinter) return;
  const on = window.K1ttyWinter.toggle();
  applyWinterUI();
  toast(t(on ? "toast_winter_on" : "toast_winter_off"), "ok");
});
$("#privacyBtn").onclick = () => setPrivacy(!PRIVACY_ON);
window.addEventListener("k1tty:languagechange", (event) => {
  setLang(event.detail?.language);
});

function selectProviderTab(prov) {
  STATE.filterProv = prov;
  const ctrl = $("#providerTabs");
  const idx = prov === "all" ? 0 : (PROV[prov]?.idx ?? 0);
  ctrl.style.setProperty("--col", String(idx % 3));
  ctrl.style.setProperty("--row", String(Math.floor(idx / 3)));
  $$(".seg-btn", ctrl).forEach((b) => {
    const act = b.dataset.prov === prov;
    b.classList.toggle("active", act);
    b.setAttribute("aria-selected", act ? "true" : "false");
  });
  renderSidebar();
  refreshStatusBadge();
}

function renderSidebar() {
  const counts = { all: STATE.allConvs.length };
  for (const pid of PROV_ORDER) counts[pid] = 0;
  for (const c of STATE.allConvs) counts[c.provider] = (counts[c.provider] || 0) + 1;
  for (const [k, v] of Object.entries(counts)) {
    const el = $("#cnt-" + k);
    if (el) el.textContent = v;
  }

  const box = $("#convList");
  const rawQ = STATE.searchQuery.trim();
  const q = rawQ.toLowerCase();

  if (fullTextMode) {
    if (rawQ.length < 2) {
      box.innerHTML = `<div class="list-empty">${esc(t("fulltext_hint"))}</div>`;
      return;
    }
    if (fullTextLoading) {
      box.innerHTML = `<div class="list-empty">${esc(t("fulltext_searching"))}</div>`;
      return;
    }
    const hits = fullTextResults.filter((r) => STATE.filterProv === "all" || r.provider === STATE.filterProv);
    if (!hits.length) {
      box.innerHTML = `<div class="list-empty">${esc(t("no_dialogs"))}</div>`;
      return;
    }
    const activeKey = STATE.active ? `${STATE.active.provider}:${STATE.active.id}` : "";
    const actTip = esc(t("actions"));
    box.innerHTML = hits.map((r) => {
      const key = `${r.provider}:${r.cid}`;
      const isAct = key === activeKey;
      const isSel = multiSelectMode && selectedConvKeys.has(key);
      const pinIco = isPinned(r.provider, r.cid) ? ico("pin", "conv-pin-ico") : "";
      return `
        <div class="conv-item${isAct ? " active" : ""}${isSel ? " is-selected" : ""}" data-prov="${esc(r.provider)}" data-cid="${esc(r.cid)}" data-hit-key="${esc(r.key || "")}" role="button" tabindex="0">
          ${multiSelectMode ? `<span class="conv-check" aria-hidden="true">${ico("check")}</span>` : ""}
          <span class="conv-ico">${aiImg(r.provider)}</span>
          <div class="conv-info">
            <div class="conv-title">${pinIco}${esc(r.title || r.cid)}</div>
            <div class="conv-snippet">${highlightSnippet(r.snippet || "", rawQ)}</div>
            <div class="conv-meta">
              <span>${esc(t("fulltext_hits", { n: r.hit_count || 1 }))}</span>
              <span>${esc(fmtShortMtime(r.mtime))}</span>
            </div>
          </div>
          <button class="conv-more" data-more="1" title="${actTip}" aria-label="${actTip}">
            ${ico("more")}
          </button>
        </div>`;
    }).join("");
    syncSidebarSelection();
    syncBatchBar();
    return;
  }

  const list = STATE.allConvs.filter((c) => {
    if (STATE.filterProv !== "all" && c.provider !== STATE.filterProv) return false;
    if (!q) return true;
    return (
      (c.title || "").toLowerCase().includes(q) ||
      (c.subtitle || "").toLowerCase().includes(q) ||
      (c.id || "").toLowerCase().includes(q)
    );
  });

  list.sort((a, b) => {
    const pa = isPinned(a.provider, a.id) ? 1 : 0;
    const pb = isPinned(b.provider, b.id) ? 1 : 0;
    if (pa !== pb) return pb - pa;
    return (Number(b.mtime_ts) || 0) - (Number(a.mtime_ts) || 0);
  });

  if (!list.length) {
    box.innerHTML = `<div class="list-empty">${esc(t("no_dialogs"))}</div>`;
    syncBatchBar();
    return;
  }

  const activeKey = STATE.active ? `${STATE.active.provider}:${STATE.active.id}` : "";
  const actTip = esc(t("actions"));
  box.innerHTML = list.map((c) => {
    const key = `${c.provider}:${c.id}`;
    const isAct = key === activeKey;
    const isSel = multiSelectMode && selectedConvKeys.has(key);
    const pinIco = isPinned(c.provider, c.id) ? ico("pin", "conv-pin-ico") : "";
    return `
      <div class="conv-item${isAct ? " active" : ""}${isSel ? " is-selected" : ""}" data-prov="${esc(c.provider)}" data-cid="${esc(c.id)}" role="button" tabindex="0">
        ${multiSelectMode ? `<span class="conv-check" aria-hidden="true">${ico("check")}</span>` : ""}
        <span class="conv-ico">${aiImg(c.provider)}</span>
        <div class="conv-info">
          <div class="conv-title">${pinIco}${esc(c.title)}</div>
          <div class="conv-meta">
            <span>${fmtSize(c.size)}</span>
            <span>${esc(fmtShortMtime(c.mtime))}</span>
            ${c.badge ? `<span class="conv-sub">${esc(localizeBadge(c.badge))}</span>` : ""}
          </div>
        </div>
        <button class="conv-more" data-more="1" title="${actTip}" aria-label="${actTip}">
          ${ico("more")}
        </button>
      </div>`;
  }).join("");
  syncSidebarSelection();
  syncBatchBar();
}

function syncSidebarSelection() {
   
   
  const active = STATE.active;
  for (const row of $("#convList").children) {
    if (!row.dataset.cid) continue;
    const selected = !!active && !active.readonlyBackup
      && row.dataset.prov === active.provider && row.dataset.cid === active.id;
    if (row.classList.contains("active") !== selected) row.classList.toggle("active", selected);
    if (selected) row.setAttribute("aria-current", "true");
    else row.removeAttribute("aria-current");
  }
}

function renderOverview() {
  const order = PROV_ORDER;
  const statsEl = $("#ovStats");
  const shareBarEl = $("#ovShareBar");
  const shareLegEl = $("#ovShareLegend");
  const gridEl = $("#provGrid");
  const recentEl = $("#ovRecent");
  const totalSizeEl = $("#ovTotalSize");
  const recentMetaEl = $("#ovRecentMeta");

  const byProv = {};
  let totalBytes = 0;
  let maxBytes = 0;
  for (const pid of order) byProv[pid] = { count: 0, bytes: 0 };
  for (const c of STATE.allConvs) {
    if (!byProv[c.provider]) continue;
    const sz = Number(c.size) || 0;
    byProv[c.provider].count += 1;
    byProv[c.provider].bytes += sz;
    totalBytes += sz;
    if (sz > maxBytes) maxBytes = sz;
  }

  const loadedProvs = order.filter((pid) => !!STATE.providers[pid]);
  const availCount = loadedProvs.filter((pid) => STATE.providers[pid].available).length;
  const blockedCount = loadedProvs.filter((pid) => STATE.providers[pid].blocked).length;
  const runningProcs = [];
  const runningNames = [];
  for (const pid of loadedProvs) {
    const st = STATE.providers[pid];
    const r = st.running;
    if (st.available && Array.isArray(r) && r.length) {
      runningProcs.push(...r);
      runningNames.push(PROV[pid].name);
    }
  }

  if (!loadedProvs.length) {
    statsEl.innerHTML = [0, 1, 2, 3].map(() =>
      `<div class="ov-stat-card"><div class="skel-bar skel-w50"></div><div class="skel-bar skel-w30"></div></div>`
    ).join("");
    gridEl.innerHTML = order.map(() =>
      `<div class="skel-item"><div class="skel-bar skel-w50"></div><div class="skel-bar skel-w90"></div></div>`
    ).join("");
    recentEl.innerHTML = `<div class="skel-item"><div class="skel-bar skel-w75"></div><div class="skel-bar skel-w50"></div></div>`;
    return;
  }

  const bkSum = STATE.backupsSummary;
  const bkByProv = {};
  for (const pid of order) bkByProv[pid] = { count: 0, bytes: 0 };
  if (bkSum && Array.isArray(bkSum.providers)) {
    for (const p of bkSum.providers) {
      if (bkByProv[p.provider]) {
        bkByProv[p.provider] = { count: Number(p.count) || 0, bytes: Number(p.bytes) || 0 };
      }
    }
  }
  const bkTotalCount = Number(bkSum?.total_count) || 0;
  const bkTotalBytes = Number(bkSum?.total_bytes) || 0;

  const procSub = blockedCount > 0
    ? t("stat_busy_files")
    : (runningNames.length > 0 ? runningNames.join(", ") : t("stat_no_procs"));

  statsEl.innerHTML = `
    <div class="ov-stat-card">
      <div class="ov-stat-label">${esc(t("stat_dialogs"))}</div>
      <div class="ov-stat-val">${STATE.allConvs.length}</div>
      <div class="ov-stat-sub">${esc(t("stat_storages", { a: availCount, b: order.length }))}</div>
    </div>
    <div class="ov-stat-card">
      <div class="ov-stat-label">${esc(t("stat_disk"))}</div>
      <div class="ov-stat-val">${fmtSize(totalBytes)}</div>
      <div class="ov-stat-sub">${esc(maxBytes > 0 ? t("stat_largest", { v: fmtSize(maxBytes) }) : t("stat_no_files"))}</div>
    </div>
    <button type="button" class="ov-stat-card is-clickable" data-open-backups="1">
      <div class="ov-stat-label">${esc(t("stat_backups"))}</div>
      <div class="ov-stat-val">${fmtSize(bkTotalBytes)}</div>
      <div class="ov-stat-sub">${esc(bkTotalCount > 0 ? t("stat_backups_sub", { n: bkTotalCount }) : t("stat_backups_empty"))}</div>
    </button>
    <div class="ov-stat-card">
      <div class="ov-stat-label">${esc(t("stat_procs"))}</div>
      <div class="ov-stat-val">${runningProcs.length}</div>
      <div class="ov-stat-sub">${esc(procSub)}</div>
    </div>`;

  if (totalSizeEl) totalSizeEl.textContent = fmtSize(totalBytes);

  const denom = totalBytes > 0 ? totalBytes : Math.max(1, STATE.allConvs.length);
  shareBarEl.innerHTML = order.map((pid) => {
    const val = totalBytes > 0 ? byProv[pid].bytes : byProv[pid].count;
    if (val <= 0) return "";
    const pct = (val / denom) * 100;
    return `<div class="share-seg prov-${pid}" style="width: ${pct.toFixed(2)}%" title="${PROV[pid].name}: ${fmtSize(byProv[pid].bytes)}"></div>`;
  }).join("");

  shareLegEl.innerHTML = order.map((pid) => {
    const b = byProv[pid].bytes;
    const rawPct = totalBytes > 0 ? (b / totalBytes) * 100 : 0;
    const pctTxt = b > 0 && rawPct < 1 ? "<1%" : `${Math.round(rawPct)}%`;
    return `
      <span class="share-leg-item">
        <span class="share-dot prov-${pid}"></span>
        <span>${PROV[pid].name}</span>
        <span class="share-leg-val">${fmtSize(b)} (${pctTxt})</span>
      </span>`;
  }).join("");

  gridEl.innerHTML = order.map((pid) => {
    const st = STATE.providers[pid] || { available: false, appdata: "", running: [] };
    const meta = PROV[pid];
    const info = byProv[pid];
    let dotCls = "ok";
    let statusTxt = t("status_ready");
    if (!st.available) {
      dotCls = "";
      statusTxt = t("status_not_found");
    } else if (st.blocked) {
      dotCls = "err";
      statusTxt = t("status_busy");
    } else if (st.running && st.running.length) {
      dotCls = "warn";
      statusTxt = t("status_active");
    } else if (!st.writable) {
      dotCls = "warn";
      statusTxt = t("status_ro");
    }
    const cleanNote = localizeProviderNote(st);
    return `
      <button type="button" class="prov-row" data-filter-prov="${pid}">
        <span class="prov-row-ico">${aiImg(pid)}</span>
        <span class="prov-row-main">
          <span class="prov-row-top">
            <span class="prov-row-name">${meta.name}</span>
            <span class="prov-status"><span class="status-dot ${dotCls}"></span><span>${esc(statusTxt)}</span></span>
          </span>
          <span class="prov-row-note">${esc(cleanNote)}</span>
          <span class="prov-row-path" data-reveal-path="${escAttr(st.appdata)}" title="${escAttr(t("reveal_in_explorer") + ": " + st.appdata)}">${ico("folder", "prov-path-ico")}<span>${esc(st.appdata)}</span></span>
        </span>
        <span class="prov-row-nums">
          <span class="prov-row-cnt">${info.count}</span>
          <span class="prov-row-size">${fmtSize(info.bytes)}</span>
        </span>
      </button>`;
  }).join("");

  const recent = STATE.allConvs.slice(0, 6);
  if (recentMetaEl) recentMetaEl.textContent = recent.length ? t("recent_of", { a: recent.length, b: STATE.allConvs.length }) : "";
  if (!recent.length) {
    recentEl.innerHTML = `<div class="list-empty">${esc(t("no_dialogs_yet"))}</div>`;
  } else {
    recentEl.innerHTML = recent.map((c) => `
      <button type="button" class="recent-row" data-open-prov="${esc(c.provider)}" data-open-cid="${esc(c.id)}">
        <span class="recent-ico">${aiImg(c.provider)}</span>
        <span class="recent-main">
          <div class="recent-title">${esc(c.title)}</div>
          <div class="recent-meta">${esc(c.mtime)}</div>
        </span>
        <span class="recent-size">${fmtSize(c.size)}</span>
      </button>`).join("");
  }

  const bkMetaEl = $("#ovBkTotalMeta");
  const bkCleanBtn = $("#ovBkCleanBtn");
  const splitEl = $("#ovDiskSplit");
  const bkShareWrap = $("#ovBkShareWrap");
  const bkShareBar = $("#ovBkShareBar");
  const bkShareLeg = $("#ovBkShareLegend");
  const bkGridEl = $("#ovBkGrid");

  if (bkMetaEl) {
    bkMetaEl.textContent = bkTotalCount > 0
      ? `${fmtSize(bkTotalBytes)} · ${t("bk_count_lbl", { n: bkTotalCount })}`
      : fmtSize(0);
  }
  if (bkCleanBtn) {
    bkCleanBtn.classList.toggle("hidden", bkTotalCount <= 0);
  }
  applyAutoBackupUI();

  if (splitEl) {
    const combined = totalBytes + bkTotalBytes;
    const chatPct = combined > 0 ? (totalBytes / combined) * 100 : 100;
    const bkPct = combined > 0 ? (bkTotalBytes / combined) * 100 : 0;
    const chatPctTxt = totalBytes > 0 && chatPct < 1 ? "<1%" : `${Math.round(chatPct)}%`;
    const bkPctTxt = bkTotalBytes > 0 && bkPct < 1 ? "<1%" : `${Math.round(bkPct)}%`;
    splitEl.innerHTML = `
      <div class="disk-split-head">
        <div class="disk-split-items">
          <span class="disk-split-item">
            <span class="disk-split-dot chats"></span>
            <span>${esc(t("disk_chats"))}</span>
            <span class="disk-split-val">${fmtSize(totalBytes)} (${chatPctTxt})</span>
          </span>
          <span class="disk-split-item">
            <span class="disk-split-dot backups"></span>
            <span>${esc(t("disk_backups"))}</span>
            <span class="disk-split-val">${fmtSize(bkTotalBytes)} (${bkPctTxt})</span>
          </span>
        </div>
        <span class="disk-split-total">${esc(t("disk_total", { v: fmtSize(combined) }))}</span>
      </div>
      <div class="disk-split-bar">
        ${totalBytes > 0 ? `<div class="disk-split-seg chats" style="width:${chatPct.toFixed(2)}%" title="${esc(t("disk_chats"))}: ${fmtSize(totalBytes)}"></div>` : ""}
        ${bkTotalBytes > 0 ? `<div class="disk-split-seg backups" style="width:${bkPct.toFixed(2)}%" title="${esc(t("disk_backups"))}: ${fmtSize(bkTotalBytes)}"></div>` : ""}
      </div>`;
  }

  if (bkShareWrap && bkShareBar && bkShareLeg) {
    bkShareWrap.classList.toggle("hidden", bkTotalBytes <= 0 && bkTotalCount <= 0);
    const bkDenom = bkTotalBytes > 0 ? bkTotalBytes : Math.max(1, bkTotalCount);
    bkShareBar.innerHTML = order.map((pid) => {
      const val = bkTotalBytes > 0 ? bkByProv[pid].bytes : bkByProv[pid].count;
      if (val <= 0) return "";
      const pct = (val / bkDenom) * 100;
      return `<div class="share-seg prov-${pid}" style="width: ${pct.toFixed(2)}%" title="${PROV[pid].name}: ${fmtSize(bkByProv[pid].bytes)}"></div>`;
    }).join("");
    bkShareLeg.innerHTML = order.map((pid) => {
      const b = bkByProv[pid].bytes;
      const c = bkByProv[pid].count;
      if (b <= 0 && c <= 0) return "";
      const rawPct = bkTotalBytes > 0 ? (b / bkTotalBytes) * 100 : 0;
      const pctTxt = b > 0 && rawPct < 1 ? "<1%" : `${Math.round(rawPct)}%`;
      return `
        <span class="share-leg-item">
          <span class="share-dot prov-${pid}"></span>
          <span>${PROV[pid].name}</span>
          <span class="share-leg-val">${fmtSize(b)} (${pctTxt})</span>
        </span>`;
    }).join("");
  }

  if (bkGridEl) {
    bkGridEl.innerHTML = order.filter((pid) => bkByProv[pid].count > 0 || bkByProv[pid].bytes > 0).map((pid) => {
      const info = bkByProv[pid];
      const cntTxt = info.count > 0 ? t("bk_count_lbl", { n: info.count }) : t("bk_none_short");
      return `
        <button type="button" class="ov-bk-card${info.count ? "" : " is-empty"}" data-bk-prov="${pid}">
          ${aiImg(pid)}
          <span class="ov-bk-card-name">${PROV[pid].name}</span>
          <span class="ov-bk-count">${esc(cntTxt)}</span>
          <span class="ov-bk-card-size">${fmtSize(info.bytes)}</span>
        </button>`;
    }).join("") || `<p class="ov-backup-empty">${esc(t("stat_backups_empty"))}</p>`;
  }
}

function showOverview() {
   
  STATE.loadingThreadToken++;
  resetThreadTransition();
  STATE.active = null;
  if (WS_READY) rpc("set_view", "all", {}).catch(() => {});
  history.replaceState(null, "", "#");
  $("#overviewPane").classList.remove("hidden");
  $("#chatWrap").classList.add("hidden");
  $("#backupBanner").classList.add("hidden");
  $("#threadStatusBtn").classList.add("hidden");
  $("#threadTokenPill")?.classList.add("hidden");
  $("#threadSlimBtn")?.classList.add("hidden");
  $("#threadFindBtn")?.classList.add("hidden");
  closeThreadSearch();
  $("#threadLockBtn").classList.add("hidden");
  $("#threadNoticesBtn").classList.add("hidden");
  $("#threadProvBadge").classList.add("hidden");
  $("#toggleSysBtn").classList.add("hidden");
  $("#threadConvertBtn").classList.add("hidden");
  $("#threadExportBtn").classList.add("hidden");
  $("#threadBackupsBtn").classList.add("hidden");
  $("#threadReloadBtn").classList.add("hidden");
  $("#topThreadSep").classList.add("hidden");
  $("#unlockerCard")?.classList.add("hidden");
  const titleEl = $("#threadTitle");
  titleEl.textContent = t("overview");
  titleEl.title = "";
  titleEl.classList.remove("is-thread");
  syncSidebarSelection();
  renderOverview();
  refreshStatusBadge();
}

let ctxTarget = null;

function openCtxMenu(x, y, prov, cid, rowEl) {
  closeCtxMenu();
  const conv = STATE.allConvs.find((c) => c.provider === prov && c.id === cid);
  ctxTarget = { provider: prov, cid, title: conv ? conv.title : cid, el: rowEl };
  rowEl?.classList.add("ctx-open");

  const menu = $("#ctxMenu");
  const pinBtn = $("#ctxPinBtn");
  if (pinBtn) {
    const pinned = isPinned(prov, cid);
    pinBtn.querySelector("span").textContent = t(pinned ? "unpin_chat" : "pin_chat");
  }
  const lockBtn = $("#ctxLockBtn");
  if (lockBtn) lockBtn.classList.toggle("hidden", prov !== "codex");
  $("#ctxNoticesBtn").classList.toggle("hidden", prov !== "claude");
  $("#ctxNoticesBtn").disabled = conv?.writable === false;
  const delBtn = $("#ctxDeleteBtn");
  disarmButton(delBtn);
  delBtn.disabled = false;
  delBtn.innerHTML = `${ico("trash")}<span class="arm-label">${esc(t("delete_conv"))}</span><span class="arm-bar"></span>`;

  menu.classList.remove("hidden");
  const mw = menu.offsetWidth || 216;
  const mh = menu.offsetHeight || 190;
  const px = Math.min(x, window.innerWidth - mw - 12);
  const py = Math.min(y, window.innerHeight - mh - 12);
  menu.style.left = Math.max(8, px) + "px";
  menu.style.top = Math.max(8, py) + "px";
}

function closeCtxMenu() {
  const menu = $("#ctxMenu");
  if (!menu.classList.contains("hidden")) {
    menu.classList.add("hidden");
    disarmButton($("#ctxDeleteBtn"));
  }
  ctxTarget?.el?.classList.remove("ctx-open");
  ctxTarget = null;
}

$("#convList").addEventListener("contextmenu", (ev) => {
  const item = ev.target.closest(".conv-item");
  if (!item) return;
  ev.preventDefault();
  openCtxMenu(ev.clientX, ev.clientY, item.dataset.prov, item.dataset.cid, item);
});

$("#ctxMenu").addEventListener("click", (ev) => {
  const btn = ev.target.closest("[data-ctx]");
  if (!btn || !ctxTarget) return;
  const action = btn.dataset.ctx;
  const { provider, cid, title, el } = ctxTarget;

  if (action === "open") {
    closeCtxMenu();
    openThread(provider, cid, { preserveSys: false });
  } else if (action === "pin") {
    closeCtxMenu();
    togglePin(provider, cid);
  } else if (action === "select") {
    closeCtxMenu();
    setMultiSelect(true);
    toggleSelectConv(provider, cid, false);
  } else if (action === "export") {
    closeCtxMenu();
    openExportModal(provider, cid, title);
  } else if (action === "convert") {
    closeCtxMenu();
    openConvertModal(provider, cid, title);
  } else if (action === "backups") {
    closeCtxMenu();
    openBackups(provider, cid);
  } else if (action === "copy-id") {
    closeCtxMenu();
    window.K1ttyDesktop.writeText(cid).then(() => toast(t("toast_id_copied"), "ok"));
  } else if (action === "lock-chat") {
    closeCtxMenu();
    if (!guardNotRunning(provider)) return;
    openLockModal(provider, cid);
  } else if (action === "system-notices") {
    closeCtxMenu();
    openNoticeModal(provider, cid);
  } else if (action === "delete-conv") {
    ev.stopPropagation();
    if (!guardNotRunning(provider)) { closeCtxMenu(); return; }
    handleArmedClick(btn, t("confirm_delete"), () => executeDeleteConv(provider, cid, el));
  }
});

document.addEventListener("click", (ev) => {
  if (!ev.target.closest("#ctxMenu") && !ev.target.closest("[data-more]")) {
    closeCtxMenu();
  }
});

async function executeDeleteConv(provider, cid, rowEl) {
  closeCtxMenu();
  rowEl?.classList.add("removing");
  try {
    const r = await rpc("delete_conv", provider, { cid, confirm: true });
    const job = await waitJob(r.job_id);
    const bdir = job.result?.backup;

    STATE.allConvs = STATE.allConvs.filter((c) => !(c.provider === provider && c.id === cid));
    renderSidebar();
    if (STATE.active && STATE.active.provider === provider && STATE.active.id === cid) {
      showOverview();
    } else {
      if (!STATE.active) renderOverview();
    }

    toast(
      t("toast_conv_deleted"),
      "ok",
      bdir ? {
        label: t("toast_undo"),
        onClick: async () => {
          try {
            const rr = await rpc("restore", provider, { backup_dir: bdir, confirm: true });
            await waitJob(rr.job_id);
            toast(t("toast_conv_restored"), "ok");
            await refreshAll(true);
            openThread(provider, cid, { preserveSys: false });
          } catch (e) {
            toast(e.message, "err");
          }
        },
      } : null
    );
  } catch (e) {
    rowEl?.classList.remove("removing");
    toast(e.message, "err");
  }
}

function toolIconFor(it) {
  if (it.subtype === "thinking" || it.tool === "thinking" || it.tool === "reasoning") return "layers";
  if (it.subtype === "tool_result") return "file-text";
  if (it.subtype === "tool_use" || (it.tool || "").toLowerCase() === "bash" || (it.tool || "").toLowerCase() === "shell") return "terminal";
  return "wrench";
}

function actionsBarHTML(it) {
  if (STATE.active && STATE.active.readonlyBackup) return "";
  const acts = it.actions || [];
  if (!acts.length) return "";
  const k = esc(it.key);
  const btns = [];
  if (acts.includes("edit")) {
    btns.push(`<button class="btn sm" data-act="edit" data-key="${k}">${ico("edit")}<span>${esc(t("edit"))}</span></button>`);
  }
  if (acts.includes("del_single")) {
    btns.push(`<button class="btn sm warn arm-btn" data-act="del-single" data-key="${k}">${ico("x")}<span class="arm-label">${esc(t("delete"))}</span><span class="arm-bar"></span></button>`);
  }
  if (acts.includes("cut_head")) {
    btns.push(`<button class="btn sm warn" data-act="cut-head" data-key="${k}">${ico("arrow-up")}<span>${esc(t("cut_above"))}</span></button>`);
  }
  if (acts.includes("cut_tail")) {
    btns.push(`<button class="btn sm danger" data-act="cut-tail" data-key="${k}">${ico("scissors")}<span>${esc(t("cut_below"))}</span></button>`);
  }
  return `<div class="msg-actions">${btns.join("")}</div>`;
}

function renderItemHTML(it, provId) {
  const k = esc(it.key);
  const meta = PROV[provId] || { name: "AI" };

  if (it.kind === "user" || it.kind === "assistant") {
    const isUser = it.kind === "user";
    const who = isUser ? t("you") : meta.name;
    const chip = isUser ? ico("user") : aiImg(provId);
    const long = (it.text || "").length > (isUser ? 900 : 2400);
    const ghost = it.in_db === false;
    return `
      <article class="msg-block kind-${it.kind}${ghost ? " ghost" : ""}" data-key="${k}">
        <div class="msg-head">
          <span class="role-chip">${chip}</span>
          <span class="msg-who">${esc(who)}</span>
          ${!isUser && it.tool ? `<span class="mono-tag">${esc(it.tool)}</span>` : ""}
          ${it.ts ? `<span class="msg-time">${fmtTime(it.ts)}</span>` : ""}
          ${ghost ? `<span class="mono-tag warn">${esc(t("transcript_only"))}</span>` : ""}
        </div>
        <div class="msg-body${long ? " clamped" : ""}">${fmtMarkdown(it.text)}</div>
        ${long ? `<div class="clamp-bar"><button class="btn sm" data-act="clamp">${ico("chev-down")}<span>${esc(t("show_more"))}</span></button></div>` : ""}
        ${actionsBarHTML(it)}
      </article>`;
  }

  if (it.kind === "tool") {
    const iconName = toolIconFor(it);
    const isErr = String(it.status || "").toLowerCase() === "error";
    const name = localizeKind(it.label || it.tool || t("tool"));
    const isThinking = String(it.subtype || "").toLowerCase() === "thinking"
      || String(it.tool || "").toLowerCase() === "thinking"
      || String(it.label || "").toLowerCase() === "reasoning";
    const bodyHtml = isThinking ? fmtMarkdown(it.text) : fmtToolText(it.text);
    return `
      <div class="tool-block${isErr ? " is-error" : ""}" data-key="${k}">
        <details>
          <summary>
            ${ico("chev-right", "tool-chev")}
            ${ico(iconName, "tool-ico")}
            <span class="tool-name">${esc(name)}</span>
            ${isErr ? `<span class="mono-tag err">${esc(t("error"))}</span>` : ""}
            <span class="spacer"></span>
            ${it.ts ? `<span class="msg-time">${fmtTime(it.ts)}</span>` : ""}
          </summary>
          <div class="tool-body${isThinking ? " is-markdown" : ""}">${bodyHtml}</div>
          ${actionsBarHTML(it)}
        </details>
      </div>`;
  }

  const isErr = it.kind === "error";
  const noticeType = provId === "claude" && ["info", "notice", "suggestion", "warning", "safety_classifier"].includes(it.notice_type) ? it.notice_type : "";
  const isGuard = noticeType === "safety_classifier" || (!noticeType && it.subtype === "safeguard");
  const iconName = isErr ? "alert" : (isGuard ? "shield" : noticeType === "warning" ? "warn" : "info");
  const long = (it.text || "").length > 900;
  const noticeTitle = noticeType
    ? noticeType === "safety_classifier" ? t("notices_safety_label") : t("notices_" + noticeType)
    : isErr ? localizeUnlockerCategory(localizeKind(it.label || t("error"))) : localizeKind(it.label || t("system"));
  return `
    <article class="notice-block${isErr ? " is-error" : ""}${noticeType ? " notice-" + noticeType : ""}" data-key="${k}">
      <div class="msg-head">
        <span class="role-chip">${ico(iconName)}</span>
        <span class="msg-who">${esc(noticeTitle)}</span>
        ${it.code ? `<span class="mono-tag">${esc(it.code)}</span>` : ""}
        ${it.ts ? `<span class="msg-time">${fmtTime(it.ts)}</span>` : ""}
      </div>
      <div class="msg-body${long ? " clamped" : ""}">${fmtMarkdown(it.text)}</div>
      ${long ? `<div class="clamp-bar"><button class="btn sm" data-act="clamp">${ico("chev-down")}<span>${esc(t("show_more"))}</span></button></div>` : ""}
      ${actionsBarHTML(it)}
    </article>`;
}

function renderTurnsHTML(turns, provId) {
  const parts = [];
  for (const turn of turns) {
    for (const it of turn.items) parts.push(renderItemHTML(it, provId));
  }
  return parts.join("");
}

let activeStatusNote = "";

function syncTopThreadSep() {
  const sep = $("#topThreadSep");
  if (!sep) return;
  const ids = [
    "#threadTokenPill",
    "#threadSlimBtn",
    "#threadFindBtn",
    "#threadStatusBtn",
    "#threadLockBtn",
    "#threadNoticesBtn",
    "#toggleSysBtn",
    "#threadConvertBtn",
    "#threadExportBtn",
    "#threadBackupsBtn",
    "#threadReloadBtn",
  ];
  const anyVis = ids.some((sel) => {
    const el = $(sel);
    return el && !el.classList.contains("hidden");
  });
  sep.classList.toggle("hidden", !anyVis);
}

function statusTargetProvider() {
  if (STATE.active && STATE.active.readonlyBackup) return null;
  if (STATE.filterProv && STATE.filterProv !== "all") return STATE.filterProv;
  if (STATE.active) return STATE.active.provider;
  return null;
}

function isConvLocked(provId, cid = null) {
  if (STATE.active && !STATE.active.readonlyBackup && STATE.active.provider === provId) {
    if (!cid || STATE.active.id === cid) {
      if (STATE.active.fileLocked) return true;
      if (STATE.active.liveUnlocked) return false;
    }
  }
  return !!STATE.providers[provId]?.blocked;
}

function guardNotRunning(provId, cid = null) {
  if (!isConvLocked(provId, cid)) return true;
  const pname = PROV[provId]?.name || STATE.providers[provId]?.name || provId;
  toast(t("toast_close_app_first", { name: pname }), "err");
  return false;
}

let dbUnlockActionPending = false;

function syncRunningGuard() {
  const wrap = $("#chatWrap");
  const banner = $("#runningGuardBanner");
  const txtEl = $("#runningGuardText");
  const iconEl = $("#runningGuardIcon");
  const actionsEl = $("#runningGuardActions");
  const unlockBtn = $("#runningUnlockDbBtn");
  const restartBtn = $("#runningRestartHubBtn");
  const relockBtn = $("#runningRelockDbBtn");
  if (!wrap || !banner || !txtEl) return;
  if (!STATE.active || STATE.active.readonlyBackup) {
    wrap.classList.remove("app-locked");
    banner.classList.add("hidden");
    banner.classList.remove("is-live-unlocked");
    if (actionsEl) actionsEl.classList.add("hidden");
    return;
  }
  const pid = STATE.active.provider;
  const locked = isConvLocked(pid, STATE.active.id);
  const canUnlock = !!(STATE.active.canUnlockDb || pid === "antigravity" || pid === "antigravity_cli");
  const canRestart = !!(STATE.active.canRestartHub || pid === "antigravity" || pid === "antigravity_cli");
  const liveUnlockedActive = !locked && !!STATE.active.liveUnlocked && !!STATE.active.presenceLocked;
  const pname = PROV[pid]?.name || pid;

  wrap.classList.toggle("app-locked", locked);
  if (locked) {
    banner.classList.remove("hidden", "is-live-unlocked");
    if (iconEl) iconEl.innerHTML = '<use href="#i-lock"/>';
    txtEl.textContent = t(canUnlock ? "running_guard_unlock_banner" : "running_guard_banner", { name: pname });
    if (actionsEl) actionsEl.classList.toggle("hidden", !canUnlock && !canRestart);
    if (unlockBtn) {
      unlockBtn.classList.toggle("hidden", !canUnlock);
      unlockBtn.disabled = dbUnlockActionPending;
    }
    if (restartBtn) {
      restartBtn.classList.toggle("hidden", !canRestart);
      restartBtn.disabled = dbUnlockActionPending;
    }
    if (relockBtn) relockBtn.classList.add("hidden");
  } else if (liveUnlockedActive) {
    banner.classList.remove("hidden");
    banner.classList.add("is-live-unlocked");
    if (iconEl) iconEl.innerHTML = '<use href="#i-unlock"/>';
    txtEl.textContent = t("live_unlocked_banner", { name: pname });
    if (actionsEl) actionsEl.classList.remove("hidden");
    if (unlockBtn) unlockBtn.classList.add("hidden");
    if (restartBtn) {
      restartBtn.classList.toggle("hidden", !canRestart);
      restartBtn.disabled = dbUnlockActionPending;
    }
    if (relockBtn) {
      relockBtn.classList.remove("hidden");
      relockBtn.disabled = dbUnlockActionPending;
    }
  } else {
    banner.classList.add("hidden");
    banner.classList.remove("is-live-unlocked");
    if (actionsEl) actionsEl.classList.add("hidden");
  }
  const lockBtn = $("#threadLockBtn");
  if (lockBtn) lockBtn.disabled = locked;
  updateThreadNoticesBtn();
  const codexUnlockBtn = $("#unlockerApplyBtn");
  if (codexUnlockBtn) codexUnlockBtn.disabled = locked;
}

function applyConvLockPayload(target, msg) {
  if (!target || !msg) return;
  if ("locked" in msg) target.fileLocked = !!msg.locked;
  if ("file_locked" in msg) target.fileLocked = !!msg.file_locked;
  if ("reason" in msg) target.lockReason = msg.reason || "";
  if ("lock_reason" in msg) target.lockReason = msg.lock_reason || "";
  if ("live_unlocked" in msg) target.liveUnlocked = !!msg.live_unlocked;
  if ("presence_locked" in msg) target.presenceLocked = !!msg.presence_locked;
  if ("can_unlock_db" in msg) target.canUnlockDb = !!msg.can_unlock_db;
  if ("can_restart_hub" in msg) target.canRestartHub = !!msg.can_restart_hub;
}

function onConvLockUpdate(msg) {
  if (!STATE.active || STATE.active.readonlyBackup) return;
  if (STATE.active.provider !== msg.provider || STATE.active.id !== msg.cid) return;
  applyConvLockPayload(STATE.active, msg);
  refreshStatusBadge();
}

async function runLiveUnlockOp(op) {
  if (dbUnlockActionPending || !STATE.active || STATE.active.readonlyBackup) return;
  const { provider, id } = STATE.active;
  const pname = PROV[provider]?.name || provider;
  dbUnlockActionPending = true;
  syncRunningGuard();
  try {
    const res = await rpc(op, provider, { cid: id });
    if (STATE.active && STATE.active.provider === provider && STATE.active.id === id) {
      applyConvLockPayload(STATE.active, res);
    }
    refreshStatusBadge();
    if (op === "unlock_db") {
      toast(t("toast_db_unlocked"), "ok");
    } else if (op === "relock_db") {
      toast(t("toast_db_relocked"), "ok");
    } else if (op === "restart_hub") {
      const pids = Array.isArray(res.pids) ? res.pids : [];
      if (pids.length) {
        toast(t("toast_hub_restarted", { name: pname, pids: pids.join(", ") }), "ok");
      } else {
        toast(t("toast_hub_not_found", { name: pname }));
      }
      if (STATE.active && STATE.active.provider === provider && STATE.active.id === id) {
        refreshCurrentTranscript({ force: true }).catch(() => {});
      }
    }
  } catch (err) {
    toast(err?.message || String(err), "err");
  } finally {
    dbUnlockActionPending = false;
    syncRunningGuard();
  }
}

$("#runningUnlockDbBtn")?.addEventListener("click", () => runLiveUnlockOp("unlock_db"));
$("#runningRelockDbBtn")?.addEventListener("click", () => runLiveUnlockOp("relock_db"));
$("#runningRestartHubBtn")?.addEventListener("click", () => runLiveUnlockOp("restart_hub"));

function refreshStatusBadge() {
  const pid = statusTargetProvider();
  if (!pid) {
    const btn = $("#threadStatusBtn");
    if (btn) btn.classList.add("hidden");
    activeStatusNote = "";
    syncTopThreadSep();
    syncRunningGuard();
    return;
  }
  const writable = (STATE.active && STATE.active.provider === pid)
    ? STATE.active.writable
    : (STATE.providers[pid]?.writable !== false);
  updateThreadStatus(pid, writable);
  syncRunningGuard();
}

function onProcsUpdate(delta) {
  if (!delta || typeof delta !== "object") return;
  for (const [pid, st] of Object.entries(delta)) {
    if (st && typeof st === "object") {
      STATE.providers[pid] = st;
    }
  }
  refreshStatusBadge();
  if (!STATE.active) renderOverview();
}

function updateThreadStatus(provId, writable) {
  const btn = $("#threadStatusBtn");
  const st = STATE.providers[provId];
  if (!st) {
    btn.classList.add("hidden");
    activeStatusNote = "";
    syncTopThreadSep();
    return;
  }

  let show = false, cls = "", iconName = "app-window", pillTxt = "", msg = "";
  const pname = PROV[provId]?.name || st.name || provId;
  const activeFileLocked = !!(STATE.active && !STATE.active.readonlyBackup && STATE.active.provider === provId && STATE.active.fileLocked);
  const activeLiveUnlocked = !!(STATE.active && !STATE.active.readonlyBackup && STATE.active.provider === provId && !activeFileLocked && STATE.active.liveUnlocked && STATE.active.presenceLocked);
  if (activeFileLocked || (st.blocked && !activeLiveUnlocked)) {
    show = true;
    cls = "err";
    iconName = "lock";
    pillTxt = t("pill_blocked", { name: pname });
    msg = t("toast_close_app_first", { name: pname });
  } else if (activeLiveUnlocked) {
    show = true;
    cls = "running";
    iconName = "unlock";
    pillTxt = t("pill_live_unlocked");
    msg = t("live_unlocked_banner", { name: pname });
  } else if (st.available && st.running && st.running.length) {
    show = true;
    cls = "running";
    iconName = "app-window";
    pillTxt = t("pill_running", { name: pname });
    msg = localizeProviderNote(st);
  } else if (!writable && STATE.active && STATE.active.provider === provId) {
    show = true;
    cls = "ro";
    iconName = "eye";
    pillTxt = t("status_ro");
    msg = t("status_ro");
  }
  if (show && msg && !msg.toLowerCase().includes(pname.toLowerCase())) {
    msg = `${pname} · ${msg}`;
  }
  activeStatusNote = msg;
  btn.className = "btn status-pill " + cls + (show ? "" : " hidden");
  btn.title = msg;
  btn.setAttribute("aria-label", msg || pillTxt || pname);
  btn.innerHTML = `${ico(iconName)}<span class="status-pill-txt">${esc(pillTxt)}</span>`;
  syncTopThreadSep();
}
$("#threadStatusBtn").onclick = () => {
  if (activeStatusNote) toast(activeStatusNote);
};

 
 
 
let threadTransitionTimer = null;
let transcriptRenderSequence = 0;
let transcriptEntryAnimation = null;
const transcriptMutations = new Set();
function transcriptMutationKey(provider, cid) {
  return JSON.stringify([provider, cid]);
}
function transcriptMutationPending(context = STATE.active) {
  return !!context && transcriptMutations.has(transcriptMutationKey(context.provider, context.cid ?? context.id));
}
function canMutateTranscript(context = STATE.active) {
  return !!context?.hasRendered && !context.readonlyBackup && !context.loading
    && !context.needsRefresh && !transcriptMutationPending(context);
}
function beginTranscriptMutation(context) {
  if (transcriptMutationPending(context)) return false;
  if (STATE.active?.provider === context.provider && STATE.active.id === context.cid) {
    if (!canMutateTranscript()) return false;
     
    STATE.loadingThreadToken++;
    STATE.active.transcriptCache?.clear();
  }
  transcriptMutations.add(transcriptMutationKey(context.provider, context.cid));
  return true;
}
async function finishTranscriptMutation(context, changed) {
  transcriptMutations.delete(transcriptMutationKey(context.provider, context.cid));
  if (changed) await refreshAfterTranscriptMutation(context);
}
function resetThreadTransition() {
  if (threadTransitionTimer !== null) {
    clearTimeout(threadTransitionTimer);
    threadTransitionTimer = null;
  }
  const overlay = $("#threadTransitionOverlay");
  overlay?.classList.remove("is-visible");
  overlay?.setAttribute("aria-hidden", "true");
  const wrap = $("#chatWrap");
  wrap?.removeAttribute("aria-busy");
  wrap?.removeAttribute("data-loading-kind");
  const scroll = $("#feedScroll");
  if (scroll) scroll.inert = false;
  if (STATE.active) STATE.active.loading = false;
  $("#toggleSysBtn")?.removeAttribute("aria-busy");
  updateThreadNoticesBtn();
}
function beginThreadTransition(token, { update = false } = {}) {
  transcriptEntryAnimation?.cancel();
  transcriptEntryAnimation = null;
  if (threadTransitionTimer !== null) clearTimeout(threadTransitionTimer);
  const wrap = $("#chatWrap");
  wrap?.setAttribute("aria-busy", "true");
  wrap?.setAttribute("data-loading-kind", update ? "update" : "open");
  const scroll = $("#feedScroll");
  if (scroll) {
    scroll.inert = !update;
    scroll.style.overflowAnchor = "none";
  }
  if (STATE.active) STATE.active.loading = true;
  const overlay = $("#threadTransitionOverlay");
   
  threadTransitionTimer = setTimeout(() => {
    threadTransitionTimer = null;
    if (token !== STATE.loadingThreadToken) return;
    overlay?.classList.add("is-visible");
    overlay?.setAttribute("aria-hidden", "false");
  }, 130);
}
function finishThreadTransition(token) {
  if (token === STATE.loadingThreadToken) resetThreadTransition();
}

function transcriptItems(turns) {
  return (turns || []).flatMap((turn) => turn.items || []);
}

function transcriptIdentity(item) {
   
   
  return JSON.stringify([item.kind, item.ts || "", item.text || "", item.tool || ""]);
}

function sameTranscriptItem(previous, next) {
  if (!previous) return false;
   
   
  for (const key of ["key", "kind", "text", "tool", "label", "subtype", "notice_type", "level", "status", "code", "ts", "in_db"]) {
    if (previous[key] !== next[key]) return false;
  }
  const a = previous.actions || [], b = next.actions || [];
  return a.length === b.length && a.every((action, index) => action === b[index]);
}

function transcriptClock() {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}

function yieldTranscriptWork() {
   
   
  if (typeof scheduler !== "undefined" && scheduler.yield) return scheduler.yield();
  return new Promise((resolve) => setTimeout(resolve, 0));
}

function animateTranscriptEntry() {
  transcriptEntryAnimation?.cancel();
  transcriptEntryAnimation = null;
  if (document.documentElement?.dataset.motion === "reduced") return;
  const scroll = $("#feedScroll");
  if (typeof scroll?.animate !== "function") return;
  const animation = scroll.animate([
    { opacity: .94, transform: "translateY(3px)" },
    { opacity: 1, transform: "translateY(0)" },
  ], { duration: 160, easing: "cubic-bezier(.2,.7,.2,1)" });
  transcriptEntryAnimation = animation;
   
   
  animation.finished?.catch(() => {});
  animation.onfinish = () => { if (transcriptEntryAnimation === animation) transcriptEntryAnimation = null; };
}

if (typeof window !== "undefined") window.addEventListener("k1tty:appearancechange", (event) => {
  if (event.detail?.reducedMotion) {
    transcriptEntryAnimation?.cancel();
    transcriptEntryAnimation = null;
  }
});

function captureTranscriptAnchor() {
  const scroll = $("#feedScroll");
  const top = scroll.getBoundingClientRect().top;
  const rows = Array.from($("#chatFeed").children).filter((el) => el.dataset.key);
  const measured = new Map();
  const position = (index) => {
    if (measured.has(index)) return measured.get(index);
    const el = rows[index];
    const rect = el.getBoundingClientRect();
    const result = {
      key: el.dataset.key,
      identity: el.__k1ttyIdentity,
      offset: rect.top - top,
      bottom: rect.bottom - top,
      index,
      message: el.matches(".kind-user, .kind-assistant"),
    };
    measured.set(index, result);
    return result;
  };
   
   
  let low = 0, high = rows.length;
  while (low < high) {
    const middle = (low + high) >>> 1;
    if (position(middle).bottom <= 0) low = middle + 1;
    else high = middle;
  }
  const visible = [];
  for (let index = low; index < rows.length; index++) {
    const row = position(index);
    if (row.offset >= scroll.clientHeight) break;
    visible.push(row);
    if (row.message && row.offset >= 0) break;
  }
  const first = visible.find((row) => row.message && row.offset >= 0)
    || visible.find((row) => row.message) || visible[0];
  if (!first) return { top: scroll.scrollTop, candidates: [] };
  const candidates = [first];
  for (let distance = 1; candidates.length < 13 && distance < rows.length; distance++) {
    if (first.index - distance >= 0) candidates.push(position(first.index - distance));
    if (candidates.length < 13 && first.index + distance < rows.length) candidates.push(position(first.index + distance));
  }
  return { top: scroll.scrollTop, candidates };
}

function restoreTranscriptAnchor(anchor) {
  if (!anchor) return;
  const scroll = $("#feedScroll");
  const top = scroll.getBoundingClientRect().top;
  const rows = Array.from($("#chatFeed").children);
  const byKey = new Map(rows.map((row) => [row.dataset.key, row]));
  const byIdentity = new Map();
  for (const row of rows) if (row.__k1ttyIdentity && !byIdentity.has(row.__k1ttyIdentity)) byIdentity.set(row.__k1ttyIdentity, row);
  for (const candidate of anchor.candidates) {
    const keyed = byKey.get(candidate.key);
    const el = keyed?.__k1ttyIdentity === candidate.identity ? keyed : byIdentity.get(candidate.identity);
    if (!el) continue;
    scroll.scrollTop += el.getBoundingClientRect().top - top - candidate.offset;
    return;
  }
  scroll.scrollTop = anchor.top;
}

async function commitThreadTranscript(token, turns, hasMore, { preserve = false } = {}) {
  if (token !== STATE.loadingThreadToken) return false;
  const context = STATE.active;
  const language = CUR_LANG;
  const sequence = ++transcriptRenderSequence;
  const current = () => token === STATE.loadingThreadToken && context === STATE.active && sequence === transcriptRenderSequence;
  const feed = $("#chatFeed");
  const scroll = $("#feedScroll");
  const existing = preserve ? Array.from(feed.children) : [];
  const oldRows = new Map(existing.map((el) => [el.dataset.key, el]));
  const oldIdentities = new Map();
  for (const old of existing) {
    if (!old.__k1ttyIdentity) continue;
    if (!oldIdentities.has(old.__k1ttyIdentity)) oldIdentities.set(old.__k1ttyIdentity, { rows: [], cursor: 0 });
    oldIdentities.get(old.__k1ttyIdentity).rows.push(old);
  }
  const claimed = new Set();
  const items = Array.isArray(turns) ? transcriptItems(turns) : [];
   
   
  CODE_STORE.length = 0;
  const template = document.createElement("template");
  const nextRows = [];
  const stateTransfers = [];
  let sliceStart = transcriptClock(), sliceItems = 0;
  if (items.length > 24) {
    await yieldTranscriptWork();
    if (!current()) return false;
    sliceStart = transcriptClock();
  }
  for (const item of items) {
    if (++sliceItems > 24 || transcriptClock() - sliceStart >= 6) {
      await yieldTranscriptWork();
      if (!current()) return false;
      sliceStart = transcriptClock();
      sliceItems = 0;
    }
    const keyed = oldRows.get(String(item.key));
    if (keyed && !claimed.has(keyed) && sameTranscriptItem(keyed.__k1ttyItem, item) && keyed.__k1ttyLanguage === language) {
      claimed.add(keyed);
      nextRows.push(keyed);
      continue;
    }
    const identity = transcriptIdentity(item);
    const bucket = oldIdentities.get(identity);
     
    while (bucket && bucket.cursor < bucket.rows.length && claimed.has(bucket.rows[bucket.cursor])) bucket.cursor++;
    const matched = bucket?.rows[bucket.cursor];
    const old = preserve
      ? (keyed?.__k1ttyIdentity === identity && !claimed.has(keyed) ? keyed : matched || (!claimed.has(keyed) ? keyed : null))
      : null;
    if (old) claimed.add(old);
     
    template.innerHTML = renderItemHTML(item, context.provider);
    const next = template.content.firstElementChild;
    next.__k1ttyItem = { ...item, actions: item.actions?.slice() };
    next.__k1ttyIdentity = identity;
    next.__k1ttyLanguage = language;
    if (old) stateTransfers.push({ old, next });
    nextRows.push(next);
  }
  if (!current()) return false;
   
  if (CUR_LANG !== language) return commitThreadTranscript(token, turns, hasMore, { preserve });
  if (!items.length) {
    template.innerHTML = (typeof turns === "string" && turns) || `<div class="list-empty">${esc(t("no_messages"))}</div>`;
    nextRows.push(...template.content.children);
  }
   
   
  for (const { old, next } of stateTransfers) {
    if (old.querySelector("details[open]")) next.querySelector("details")?.setAttribute("open", "");
    if (old.querySelector(".clamp-bar") && !old.querySelector(".msg-body.clamped")) {
      next.querySelector(".msg-body")?.classList.remove("clamped");
      const clamp = next.querySelector('[data-act="clamp"]');
      if (clamp) clamp.innerHTML = `${ico("chev-up")}<span>${esc(t("collapse"))}</span>`;
    }
  }
   
   
   
   
  const anchor = preserve ? captureTranscriptAnchor() : null;
   
   
  const keep = new Set(nextRows);
  for (const row of Array.from(feed.children)) if (!keep.has(row)) row.remove();
  let cursor = feed.firstElementChild;
  for (const next of nextRows) {
    if (next === cursor) cursor = cursor.nextElementSibling;
    else feed.insertBefore(next, cursor);
  }
  while (cursor) {
    const next = cursor.nextElementSibling;
    cursor.remove();
    cursor = next;
  }
  feed.classList.remove("thread-swap-preparing");
  renderMoreBar(hasMore);
  if (preserve) restoreTranscriptAnchor(anchor);
  else scroll.scrollTop = scroll.scrollHeight;
  updateScrollBtn();
  if (STATE.active) STATE.active.hasRendered = true;
  if (!preserve) animateTranscriptEntry();
  return true;
}

function requestTranscriptPage(context, includeTools, before, limit = 80) {
  const data = { limit, include_tools: includeTools };
  if (before != null) data.before = before;
  if (context.readonlyBackup) {
    data.backup_dir = context.backupDir;
    return rpc("backup_entries", context.provider, data);
  }
  data.cid = context.id;
  return rpc("entries", context.provider, data);
}

async function readTranscriptRange(context, includeTools, targetMessages, token) {
  let result = null;
  let before = null;
  let count = 0;
  const turns = [];
  const cursors = new Set();
  do {
    const page = await requestTranscriptPage(context, includeTools, before, Math.min(150, Math.max(1, targetMessages - count)));
    if (token !== STATE.loadingThreadToken || STATE.active !== context) return null;
    if (!result) result = { ...page };
    turns.unshift(...(page.turns || []));
    count += Number(page.page_msgs) || 0;
    result.cursor = page.cursor;
    result.has_more = !!page.has_more;
    before = page.cursor;
    if (!page.has_more || before == null || count >= targetMessages) break;
    if (cursors.has(String(before))) throw new Error(t("err_op"));
    cursors.add(String(before));
  } while (true);
  result.turns = turns;
  result.page_msgs = count;
  if (/^загружено:/.test(result.total_label || "")) result.total_label = `загружено: ${count}`;
  return result;
}

function rememberTranscript(context, showSys, response) {
  context.transcriptCache ||= new Map();
  context.transcriptCache.set(showSys, response);
  context.renderedShowSys = showSys;
}

async function applyTranscriptResponse(response, { preserve = false, showSys = STATE.showSys } = {}) {
  const active = STATE.active;
  const token = STATE.loadingThreadToken;
  if (!active) return false;
  if (!await commitThreadTranscript(token, response.turns || [], !!response.has_more, { preserve })) return false;
  if (active !== STATE.active || token !== STATE.loadingThreadToken) return false;
  STATE.cursor = response.cursor;
  STATE.loadedMsgs = response.page_msgs || 0;
  active.lastTurns = response.turns || [];
  active.needsRefresh = false;
  active.hasMore = !!response.has_more;
  active.totalLabel = response.total_label || "";
  if (!active.readonlyBackup) {
    active.writable = response.writable !== false;
    active.fileLocked = !!response.file_locked;
    active.lockReason = response.lock_reason || "";
    active.liveUnlocked = !!response.live_unlocked;
    active.presenceLocked = !!response.presence_locked;
    active.canUnlockDb = !!response.can_unlock_db;
    active.canRestartHub = !!response.can_restart_hub;
    active.blockedInfo = response.blocked_info || null;
    if (response.block_presets?.length) LOCK_PRESETS = response.block_presets;
    updateThreadLockBtn();
    refreshStatusBadge();
  }
  rememberTranscript(active, showSys, response);
  if (active.blockedInfo?.is_blocked) showUnlockerCard(active.blockedInfo);
  else hideUnlockerCard();
  return true;
}

async function refreshCurrentTranscript({ filterOnly = false, force = false } = {}) {
  const context = STATE.active;
  if (!context || (!context.hasRendered && !force) || transcriptMutationPending(context)) return;
  const showSys = STATE.showSys;
  const previouslyShown = context.renderedShowSys ?? showSys;
  const targetMessages = Math.max(80, STATE.loadedMsgs || 0);
  const token = ++STATE.loadingThreadToken;
  const cached = filterOnly ? context.transcriptCache?.get(showSys) : null;
  if (!filterOnly) context.transcriptCache?.clear();
  beginThreadTransition(token, { update: !!context.hasRendered });
  if (filterOnly) $("#toggleSysBtn").setAttribute("aria-busy", "true");
  try {
    const response = cached && (cached.page_msgs >= (STATE.loadedMsgs || 0) || !cached.has_more)
      ? cached
      : await readTranscriptRange(context, showSys, targetMessages, token);
    if (!response || token !== STATE.loadingThreadToken || STATE.active !== context) return;
    await applyTranscriptResponse(response, { preserve: !!context.hasRendered, showSys });
  } catch (error) {
    if (token !== STATE.loadingThreadToken || STATE.active !== context) return;
    STATE.showSys = previouslyShown;
    syncSysButton();
    if (!context.hasRendered) {
      await commitThreadTranscript(token, `<div class="note err">${ico("alert")}<span>${esc(error.message)}</span></div>`, false);
    }
    if (token !== STATE.loadingThreadToken || STATE.active !== context) return;
    toast(error.message, "err");
  } finally {
    finishThreadTransition(token);
  }
}

async function refreshAfterTranscriptMutation(context) {
  if (!STATE.active || STATE.active.readonlyBackup) return;
  if (STATE.active.provider !== context.provider || STATE.active.id !== context.cid) return;
   
   
  STATE.active.needsRefresh = true;
  await refreshCurrentTranscript({ force: true });
}

async function openThread(provider, cid, { preserveSys = true } = {}) {
  if (STATE.active?.hasRendered && !STATE.active.readonlyBackup && STATE.active.provider === provider && STATE.active.id === cid) {
    if (!preserveSys) { STATE.showSys = false; syncSysButton(); }
    return refreshCurrentTranscript();
  }
  const conv = STATE.allConvs.find((c) => c.provider === provider && c.id === cid);
  const title = conv ? conv.title : cid;
  STATE.active = { provider, id: cid, title, writable: conv ? conv.writable : true, readonlyBackup: false };
  if (!preserveSys) STATE.showSys = false;

  const token = ++STATE.loadingThreadToken;
  history.replaceState(null, "", `#${provider}/${encodeURIComponent(cid)}`);
  if (isMobile()) toggleDrawer(false);

  $("#overviewPane").classList.add("hidden");
  $("#chatWrap").classList.remove("hidden");
  $("#backupBanner").classList.add("hidden");

  const badge = $("#threadProvBadge");
  badge.innerHTML = aiImg(provider);
  badge.classList.remove("hidden");
  const titleEl = $("#threadTitle");
  titleEl.textContent = title;
  titleEl.classList.add("is-thread");
  titleEl.title = PRIVACY_ON ? "" : title;

  $("#toggleSysBtn").classList.remove("hidden");
  $("#threadConvertBtn").classList.remove("hidden");
  $("#threadExportBtn").classList.remove("hidden");
  $("#threadBackupsBtn").classList.remove("hidden");
  $("#threadReloadBtn").classList.remove("hidden");
  $("#threadFindBtn")?.classList.remove("hidden");
  closeThreadSearch();
  $("#topThreadSep").classList.remove("hidden");
  $("#threadLockBtn").classList.toggle("hidden", provider !== "codex");
  updateThreadNoticesBtn();
  syncOptionalHeaderTools();
  loadContextStats(provider, cid, token);
  hideUnlockerCard();
  syncSysButton();
  refreshStatusBadge();
  syncSidebarSelection();

  beginThreadTransition(token);
  try {
    const r = await rpc("entries", provider, { cid, limit: 80, include_tools: STATE.showSys });
    if (token !== STATE.loadingThreadToken) return;
    await applyTranscriptResponse(r);
  } catch (e) {
    if (token !== STATE.loadingThreadToken) return;
    await commitThreadTranscript(token, `<div class="note err">${ico("alert")}<span>${esc(e.message)}</span></div>`, false);
    if (token !== STATE.loadingThreadToken) return;
    $("#moreWrap").innerHTML = "";
  } finally {
    finishThreadTransition(token);
  }
}

async function openBackupResearch(provider, backupDir, backupName, title, { preserveSys = false } = {}) {
  if (STATE.active?.hasRendered && STATE.active.readonlyBackup && STATE.active.provider === provider && STATE.active.backupDir === backupDir) {
    closeModal("bkModal");
    if (!preserveSys) { STATE.showSys = false; syncSysButton(); }
    return refreshCurrentTranscript();
  }
  const prevLive = STATE.active && !STATE.active.readonlyBackup
    ? { provider: STATE.active.provider, id: STATE.active.id }
    : (STATE.active?.prevLive || null);
  STATE.active = {
    provider,
    id: backupDir,
    title: title || backupName,
    writable: false,
    readonlyBackup: true,
    backupDir,
    backupName,
    prevLive,
  };
  if (!preserveSys) STATE.showSys = false;

  const token = ++STATE.loadingThreadToken;
  closeModal("bkModal");
  if (isMobile()) toggleDrawer(false);

  $("#overviewPane").classList.add("hidden");
  $("#chatWrap").classList.remove("hidden");
  $("#backupBanner").classList.remove("hidden");
  $("#backupBannerName").textContent = backupName || "";
  syncRunningGuard();

  const badge = $("#threadProvBadge");
  badge.innerHTML = aiImg(provider);
  badge.classList.remove("hidden");
  const titleEl = $("#threadTitle");
  titleEl.textContent = title || backupName;
  titleEl.classList.add("is-thread");
  titleEl.title = PRIVACY_ON ? "" : (title || backupName);

  $("#threadStatusBtn").classList.add("hidden");
  $("#threadTokenPill")?.classList.add("hidden");
  $("#threadSlimBtn")?.classList.add("hidden");
  $("#threadFindBtn")?.classList.add("hidden");
  closeThreadSearch();
  $("#threadLockBtn").classList.add("hidden");
  $("#threadNoticesBtn").classList.add("hidden");
  $("#threadConvertBtn").classList.add("hidden");
  $("#threadExportBtn").classList.add("hidden");
  $("#threadBackupsBtn").classList.add("hidden");
  $("#threadReloadBtn").classList.add("hidden");
  $("#toggleSysBtn").classList.remove("hidden");
  $("#topThreadSep").classList.remove("hidden");
  hideUnlockerCard();
  syncSysButton();
  refreshStatusBadge();
  syncSidebarSelection();

  beginThreadTransition(token);
  try {
    const r = await rpc("backup_entries", provider, {
      backup_dir: backupDir,
      limit: 80,
      include_tools: STATE.showSys,
    });
    if (token !== STATE.loadingThreadToken) return;
    STATE.active.title = r.title || title || backupName;
    titleEl.textContent = STATE.active.title;
    titleEl.title = PRIVACY_ON ? "" : STATE.active.title;
    await applyTranscriptResponse(r);
  } catch (e) {
    if (token !== STATE.loadingThreadToken) return;
    await commitThreadTranscript(token, `<div class="note err">${ico("alert")}<span>${esc(e.message)}</span></div>`, false);
    if (token !== STATE.loadingThreadToken) return;
    $("#moreWrap").innerHTML = "";
  } finally {
    finishThreadTransition(token);
  }
}

function renderMoreBar(hasMore) {
  const mw = $("#moreWrap");
  if (hasMore) {
    mw.innerHTML = `<button class="btn" id="moreBtn">${ico("arrow-up")}<span>${esc(t("load_more"))}</span></button>`;
    $("#moreBtn").onclick = loadMore;
  } else {
    mw.innerHTML = `<span class="start-line">${esc(t("start_line"))}</span>`;
  }
}

async function loadMore() {
  const context = STATE.active;
  if (!context || context.loading || context.paging || transcriptMutationPending(context) || STATE.cursor == null) return;
  const token = STATE.loadingThreadToken;
  const before = STATE.cursor;
  const showSys = STATE.showSys;
  context.paging = true;
  const btn = $("#moreBtn");
  if (btn) {
    btn.disabled = true;
    btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("loading"))}</span>`;
  }
  try {
    const r = await requestTranscriptPage(context, showSys, before);
    if (token !== STATE.loadingThreadToken || STATE.active !== context || STATE.showSys !== showSys) return;
    const turns = [...(r.turns || []), ...(context.lastTurns || [])];
    if (!await commitThreadTranscript(token, turns, !!r.has_more, { preserve: true })) return;
    if (token !== STATE.loadingThreadToken || STATE.active !== context || STATE.showSys !== showSys) return;
    STATE.cursor = r.cursor;
    STATE.loadedMsgs += r.page_msgs || 0;
    context.hasMore = !!r.has_more;
    context.lastTurns = turns;
    context.totalLabel = /^загружено:/.test(r.total_label || "") ? `загружено: ${STATE.loadedMsgs}` : (r.total_label || context.totalLabel || "");
    context.transcriptCache?.clear();
    rememberTranscript(context, showSys, { ...r, total_label: context.totalLabel, page_msgs: STATE.loadedMsgs, turns: context.lastTurns });
  } catch (e) {
    if (token !== STATE.loadingThreadToken || STATE.active !== context) return;
    toast(e.message, "err");
    renderMoreBar(context.hasMore);
  } finally {
    context.paging = false;
  }
}

function syncSysButton() {
  $("#toggleSysBtn").classList.toggle("active-toggle", STATE.showSys);
  $("#toggleSysBtn").setAttribute("aria-pressed", String(STATE.showSys));
}

function updateScrollBtn() {
  const sc = $("#feedScroll");
  const btn = $("#scrollBottomBtn");
  if (!sc || !btn) return;
  const dist = sc.scrollHeight - sc.scrollTop - sc.clientHeight;
  const hidden = dist < 380;
  if (btn.classList.contains("hidden") !== hidden) btn.classList.toggle("hidden", hidden);
}
let scrollButtonFrame = 0;
$("#feedScroll").addEventListener("scroll", () => {
  if (!scrollButtonFrame) scrollButtonFrame = requestAnimationFrame(() => { scrollButtonFrame = 0; updateScrollBtn(); });
}, { passive: true });
function preferredScrollBehavior() {
  return document.documentElement.dataset.motion === "reduced" ? "auto" : "smooth";
}
$("#scrollBottomBtn").onclick = () => $("#feedScroll").scrollTo({ top: $("#feedScroll").scrollHeight, behavior: preferredScrollBehavior() });

let editCtx = null;
async function openEdit(key) {
  if (!canMutateTranscript()) return;
  const { provider, id } = STATE.active;
  if (!guardNotRunning(provider)) return;
  const context = editCtx = { provider, cid: id, key, revision: null, ready: false, writable: false, pending: false };
  $("#editTitle").textContent = t("loading");
  $("#editText").value = "";
  $("#editText").readOnly = true;
  $("#editApplyBtn").disabled = true;
  openModal("editModal");

  startLoad();
  try {
    const r = await rpc("edit_preview", provider, { cid: id, key });
    if (editCtx !== context || !$("#editModal").classList.contains("open")) return;
    context.revision = r.revision;
    context.ready = true;
    context.writable = !!r.writable;
    $("#editTitle").textContent = localizeKind(r.kind || t("edit"));
    $("#editText").value = r.text ?? "";
    $("#editText").readOnly = !r.writable;
    $("#editApplyBtn").disabled = !r.writable;
    $("#editText").focus();
  } catch (e) {
    if (editCtx === context && $("#editModal").classList.contains("open")) {
      closeModal("editModal");
      toast(e.message, "err");
    }
  } finally {
    stopLoad();
  }
}

async function applyEdit() {
  if (!editCtx?.ready || !editCtx.writable || editCtx.pending) return;
  const context = editCtx;
  const text = $("#editText").value;
  if (!text.trim()) { toast(t("toast_empty_text"), "err"); return; }
  if (!beginTranscriptMutation(context)) return;
  context.pending = true;
  const btn = $("#editApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("saving"))}</span>`;
  try {
    const r = await rpc("apply_edit", context.provider, {
      cid: context.cid,
      key: context.key,
      text,
      revision: context.revision,
      confirm: true,
    });
    const job = await waitJob(r.job_id);
    if (editCtx === context) closeModal("editModal");
    await finishTranscriptMutation(context, true);
    showReport(t("saved"), job.result);
  } catch (e) {
    await finishTranscriptMutation(context, false);
    toast(e.message, "err");
  } finally {
    context.pending = false;
    if (editCtx === context) {
      btn.disabled = !context.ready || !context.writable;
      btn.innerHTML = `${ico("check")}<span>${esc(t("save"))}</span>`;
    }
  }
}
$("#editApplyBtn").onclick = applyEdit;

let LOCK_PRESETS = [
  {
    id: "safety",
    title: "Safety Guard",
    code: "misalignmentPolicyViolation",
    message: "This request was blocked by our safety systems. Reason: Potentially unintended activity.",
  },
  {
    id: "cyber",
    title: "Cyber Policy",
    code: "cyberPolicy",
    message: "This content was flagged for possible cybersecurity risk. If this seems wrong, try rephrasing your request.",
  },
  {
    id: "usage",
    title: "Usage Limit",
    code: "usageLimitExceeded",
    message: "You've hit your usage limit. Upgrade to Pro or try again later.",
  },
  {
    id: "overload",
    title: "Overloaded",
    code: "serverOverloaded",
    message: "Selected model is at capacity. Please try a different model.",
  },
  {
    id: "unauth",
    title: "Unauthorized",
    code: "unauthorized",
    message: "Your access token could not be refreshed.",
  },
  {
    id: "badreq",
    title: "Bad Request",
    code: "other",
    message: '{"detail":"Bad Request"}',
  },
];

function showUnlockerCard(info, scrollIntoView = false) {
  const card = $("#unlockerCard");
  const catTag = $("#unlockerCatTag");
  const codeTag = $("#unlockerCodeTag");
  const promptEl = $("#unlockerPrompt");
  const reasonEl = $("#unlockerReasonText");
  if (!card || !reasonEl) return;
  if (catTag) {
    const cat = localizeUnlockerCategory(info.category || "");
    catTag.textContent = cat;
    catTag.classList.toggle("hidden", !cat);
  }
  if (codeTag) {
    const code = info.code || "";
    codeTag.textContent = code;
    codeTag.classList.toggle("hidden", !code);
  }
  if (promptEl) {
    promptEl.textContent = localizeUnlockerPrompt(info);
  }
  const msg = (info.message || "").trim();
  reasonEl.textContent = msg;
  reasonEl.classList.toggle("hidden", !msg);
  card.classList.remove("hidden");
  if (scrollIntoView) {
    requestAnimationFrame(() => {
      card.scrollIntoView({ behavior: "smooth", block: "end" });
    });
  }
}

function hideUnlockerCard() {
  $("#unlockerCard")?.classList.add("hidden");
}

function updateThreadLockBtn() {
  updateThreadNoticesBtn();
  const btn = $("#threadLockBtn");
  if (!btn) return;
  if (!STATE.active || STATE.active.provider !== "codex" || STATE.active.writable === false) {
    btn.classList.add("hidden");
    return;
  }
  btn.classList.remove("hidden");
  const isBlocked = !!(STATE.active.blockedInfo && STATE.active.blockedInfo.is_blocked);
  if (isBlocked) {
    btn.title = t("unlock_tip");
    btn.innerHTML = `${ico("unlock")}<span>${esc(t("unlock"))}</span>`;
  } else {
    btn.title = t("lock_tip");
    btn.innerHTML = `${ico("lock")}<span>${esc(t("lock"))}</span>`;
  }
}

$("#threadLockBtn").onclick = () => {
  if (!canMutateTranscript() || STATE.active.provider !== "codex") return;
  if (!guardNotRunning(STATE.active.provider)) return;
  if (STATE.active.blockedInfo && STATE.active.blockedInfo.is_blocked) {
    showUnlockerCard(STATE.active.blockedInfo, true);
  } else {
    openLockModal(STATE.active.provider, STATE.active.id);
  }
};

$("#unlockerDismissBtn").onclick = () => hideUnlockerCard();

$("#unlockerApplyBtn").onclick = async () => {
  if (!canMutateTranscript()) return;
  const { provider, id } = STATE.active;
  const context = { provider, cid: id };
  if (!guardNotRunning(provider)) return;
  if (!beginTranscriptMutation(context)) return;
  const btn = $("#unlockerApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("unlocking"))}</span>`;
  try {
    const r = await rpc("unlock_chat", provider, { cid: id });
    await waitJob(r.job_id);
    if (STATE.active?.provider === provider && STATE.active.id === id) hideUnlockerCard();
    toast(t("toast_unlocked"), "ok");
    await finishTranscriptMutation(context, true);
  } catch (e) {
    await finishTranscriptMutation(context, false);
    toast(e.message, "err");
  } finally {
    btn.disabled = false;
    btn.innerHTML = `${ico("unlock")}<span>${esc(t("yes"))}</span>`;
  }
};

let lockCtx = null;

function selectLockPreset(presetId) {
  const box = $("#lockPresets");
  $$(".preset-chip", box).forEach((b) => b.classList.toggle("active", b.dataset.preset === presetId));
  const reasonInput = $("#lockReasonInput");
  const codeInput = $("#lockCodeInput");
  if (presetId === "custom") {
    reasonInput.value = "";
    if (!codeInput.value.trim()) codeInput.value = "misalignmentPolicyViolation";
    reasonInput.focus();
    return;
  }
  const p = LOCK_PRESETS.find((x) => x.id === presetId);
  if (p) {
    reasonInput.value = p.message;
    codeInput.value = p.code;
  }
}

function openLockModal(provider, cid) {
  if (provider !== "codex") {
    toast(t("toast_codex_only"), "err");
    return;
  }
  lockCtx = { provider, cid };
  const box = $("#lockPresets");
  box.innerHTML =
    LOCK_PRESETS.map((p) => `<button type="button" class="preset-chip" data-preset="${esc(p.id)}">${esc(p.title)}</button>`).join("") +
    `<button type="button" class="preset-chip" data-preset="custom">${esc(t("custom_reason"))}</button>`;
  selectLockPreset(LOCK_PRESETS[0]?.id || "custom");
  openModal("lockModal");
}

$("#lockPresets").addEventListener("click", (ev) => {
  const chip = ev.target.closest(".preset-chip");
  if (!chip) return;
  selectLockPreset(chip.dataset.preset);
});

$("#lockReasonInput").addEventListener("input", () => {
  const val = $("#lockReasonInput").value;
  const matched = LOCK_PRESETS.find((p) => p.message === val);
  const id = matched ? matched.id : "custom";
  $$(".preset-chip", $("#lockPresets")).forEach((b) => b.classList.toggle("active", b.dataset.preset === id));
});

$("#lockApplyBtn").onclick = async () => {
  if (!lockCtx || lockCtx.pending) return;
  const context = lockCtx;
  const message = $("#lockReasonInput").value.trim();
  const code = $("#lockCodeInput").value.trim() || "misalignmentPolicyViolation";
  if (!message) {
    toast(t("toast_enter_reason"), "err");
    $("#lockReasonInput").focus();
    return;
  }
  if (!beginTranscriptMutation(context)) return;
  context.pending = true;
  const btn = $("#lockApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("locking"))}</span>`;
  try {
    const r = await rpc("lock_chat", context.provider, { cid: context.cid, message, code });
    await waitJob(r.job_id);
    if (lockCtx === context) closeModal("lockModal");
    toast(t("toast_locked"), "ok");
    await finishTranscriptMutation(context, true);
  } catch (e) {
    await finishTranscriptMutation(context, false);
    toast(e.message, "err");
  } finally {
    context.pending = false;
    if (lockCtx === context) {
      btn.disabled = false;
      btn.innerHTML = `${ico("lock")}<span>${esc(t("lock_apply"))}</span>`;
    }
  }
};

let noticeCtx = null;

function updateThreadNoticesBtn() {
  const context = STATE.active;
  const visible = context?.provider === "claude" && !context.readonlyBackup && context.writable !== false;
  const btn = $("#threadNoticesBtn");
  btn.classList.toggle("hidden", !visible);
  btn.disabled = !visible || !canMutateTranscript(context) || isConvLocked(context.provider, context.id);
}

function showNoticeError(error) {
  const message = error ? String(error.message || error) : "";
  $("#noticeError").textContent = message;
  $("#noticeError").classList.toggle("hidden", !message);
  $("#noticeReloadBtn").classList.toggle("hidden", !message);
}

function syncNoticeControls() {
  const context = noticeCtx;
  if (!context) return;
  const ready = context.ready && context.writable && !context.pending;
  const chosen = context.types.find((type) => type.id === context.type);
  const hasText = !!$("#noticeTextInput").value.trim() || context.type === "safety_classifier";
  const apply = $("#noticeApplyBtn");
  apply.disabled = !ready || !chosen || !hasText;
  apply.setAttribute("aria-busy", String(context.pending));
  apply.innerHTML = context.pending && context.action === "add_system_notice"
    ? `${ico("loader", "spin")}<span>${esc(t("notices_adding"))}</span>`
    : `${ico("plus")}<span>${esc(t("notices_add"))}</span>`;
  const remove = $("#noticeDeleteBtn");
  remove.disabled = !ready || !context.count;
  if (!ARMED.has(remove)) {
    remove.innerHTML = context.pending && context.action === "delete_system_notices"
      ? `${ico("loader", "spin")}<span>${esc(t("deleting"))}</span>`
      : `${ico("trash")}<span class="arm-label">${esc(t("notices_delete_all"))}</span><span class="arm-bar"></span>`;
  }
  $("#noticeTextInput").disabled = context.pending;
  $("#noticeReloadBtn").disabled = context.pending || context.reading;
  $("#noticeLoadState").textContent = context.reading ? t("notices_loading") : "";
  $("#noticeSummary").textContent = context.ready ? context.count ? t("notices_count", { n: context.count }) : t("notices_none") : "";
  const prefix = chosen?.prefix || "";
  $("#noticePrefix").textContent = prefix;
  $("#noticePrefix").classList.toggle("hidden", !prefix);
  $("#noticeTextLabel").textContent = t(prefix ? "notices_extra" : "notices_text");
  $("#noticeTextInput").placeholder = t(prefix ? "notices_extra_ph" : "notices_text_ph");
  for (const button of $$("[data-notice-type]", $("#noticeTypeOptions"))) {
    const selected = button.dataset.noticeType === context.type;
    button.setAttribute("aria-checked", String(selected));
    button.tabIndex = selected ? 0 : -1;
    button.disabled = !ready;
  }
}

function renderNoticeState() {
  if (!noticeCtx) return;
  $("#noticeTypeOptions").innerHTML = noticeCtx.types.map((type) => {
    const key = "notices_" + type.id;
    const label = I18N[CUR_LANG][key] ? t(key) : type.label || type.id;
    const icon = type.id === "safety_classifier" ? "shield" : type.id === "warning" ? "warn" : "info";
    return `<button type="button" class="notice-type" role="radio" data-notice-type="${esc(type.id)}" aria-checked="false" tabindex="-1">${ico(icon)}<span>${esc(label)}</span></button>`;
  }).join("");
  syncNoticeControls();
}

async function loadNoticeInfo(context) {
  if (noticeCtx !== context || context.reading || context.pending) return;
  const request = ++context.request;
  context.reading = true;
  context.ready = false;
  syncNoticeControls();
  try {
    const info = await rpc("system_notices_info", context.provider, { cid: context.cid });
    if (noticeCtx !== context || context.request !== request || !$("#noticeModal").classList.contains("open")) return;
    context.revision = info.revision;
    context.types = Array.isArray(info.types) ? info.types : [];
    context.count = Number(info.notice_count) || 0;
    context.writable = info.writable !== false;
    context.ready = !!info.revision;
    if (!context.writable) showNoticeError(info.lock_reason || t("status_ro"));
    if (!context.types.some((type) => type.id === context.type)) {
      context.type = context.types.find((type) => type.id === "notice")?.id || context.types[0]?.id;
    }
    renderNoticeState();
  } catch (error) {
    if (noticeCtx === context && context.request === request) showNoticeError(error);
  } finally {
    context.reading = false;
    if (noticeCtx === context && context.request === request) syncNoticeControls();
  }
}

async function openNoticeModal(provider, cid) {
  if (provider !== "claude" || transcriptMutationPending({ provider, cid })) return;
  if (STATE.active?.provider === provider && STATE.active.id === cid
      && (STATE.active.readonlyBackup || STATE.active.writable === false || !canMutateTranscript())) return;
  const conv = STATE.allConvs.find((item) => item.provider === provider && item.id === cid);
  if (conv?.writable === false || !guardNotRunning(provider)) return;
  const context = noticeCtx = { provider, cid, ready: false, pending: false, reading: false, request: 0,
    revision: null, types: [], type: "notice", count: 0, writable: false };
  $("#noticeChatTitle").textContent = conv?.title || cid;
  $("#noticeTextInput").value = "";
  showNoticeError(null);
  disarmButton($("#noticeDeleteBtn"));
  renderNoticeState();
  openModal("noticeModal");
  await loadNoticeInfo(context);
  if (noticeCtx === context && context.ready && $("#noticeModal").classList.contains("open")) {
    $("#noticeTextInput").focus({ preventScroll: true });
  }
}

async function executeNoticeChange(context, action, data = {}) {
  if (noticeCtx !== context || !context.ready || !context.writable || context.pending) return;
  if (!guardNotRunning(context.provider) || !beginTranscriptMutation(context)) return;
  context.pending = true;
  context.action = action;
  disarmButton($("#noticeDeleteBtn"));
  showNoticeError(null);
  syncNoticeControls();
  updateThreadNoticesBtn();
  let changed = false;
  try {
    const result = await rpc(action, context.provider, { cid: context.cid, revision: context.revision, confirm: true, ...data });
    const job = await waitJob(result.job_id);
    changed = job.result?.changed !== false;
    context.ready = false;
    if (noticeCtx === context) closeModal("noticeModal");
    await finishTranscriptMutation(context, changed);
    toast(action === "add_system_notice" ? t("notices_added") : t("notices_deleted", { n: job.result?.deleted_count || 0 }), "ok");
    refreshAll(false).catch(() => {});
  } catch (error) {
    await finishTranscriptMutation(context, changed);
    if (noticeCtx === context) {
      showNoticeError(error);
      context.pending = false;
      await loadNoticeInfo(context);
    } else {
      toast(error.message || String(error), "err");
    }
  } finally {
    context.pending = false;
    if (noticeCtx === context) syncNoticeControls();
    updateThreadNoticesBtn();
  }
}

async function applySystemNotice() {
  const context = noticeCtx;
  if (!context?.ready || context.pending || !context.types.some((type) => type.id === context.type)) return;
  const text = $("#noticeTextInput").value;
  if (!text.trim() && context.type !== "safety_classifier") {
    showNoticeError(t("notices_enter_text"));
    $("#noticeTextInput").focus();
    return;
  }
  await executeNoticeChange(context, "add_system_notice", { notice_type: context.type, text });
}

async function deleteSystemNotices() {
  const context = noticeCtx;
  if (!context?.ready || !context.count || context.pending) return;
  await executeNoticeChange(context, "delete_system_notices");
}

$("#threadNoticesBtn").onclick = () => {
  if (STATE.active) openNoticeModal(STATE.active.provider, STATE.active.id);
};
$("#noticeApplyBtn").onclick = applySystemNotice;
$("#noticeDeleteBtn").onclick = () => {
  if (noticeCtx?.ready && !noticeCtx.pending && noticeCtx.count) {
    handleArmedClick($("#noticeDeleteBtn"), t("confirm_delete"), deleteSystemNotices);
  }
};
$("#noticeReloadBtn").onclick = () => {
  if (!noticeCtx || noticeCtx.pending || noticeCtx.reading) return;
  showNoticeError(null);
  loadNoticeInfo(noticeCtx);
};
$("#noticeTextInput").addEventListener("input", syncNoticeControls);
$("#noticeTypeOptions").addEventListener("click", (event) => {
  const button = event.target.closest("[data-notice-type]");
  if (!button || button.disabled || !noticeCtx || noticeCtx.pending) return;
  noticeCtx.type = button.dataset.noticeType;
  syncNoticeControls();
});
$("#noticeTypeOptions").addEventListener("keydown", (event) => {
  const buttons = $$("[data-notice-type]:not(:disabled)", $("#noticeTypeOptions"));
  const current = buttons.indexOf(event.target.closest("[data-notice-type]"));
  if (current < 0 || !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
  event.preventDefault();
  const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1
    : (current + (["ArrowLeft", "ArrowUp"].includes(event.key) ? -1 : 1) + buttons.length) % buttons.length;
  buttons[next].click();
  buttons[next].focus();
});

const WELCOME_GREETS = ["Привет", "Hello", "Привіт", "Bonjour", "Hola", "こんにちは"];
const WELCOME_THEMES = ["graphite", "light", "neko_night", "neko_sakura", "neko_matcha", "manul_steppe", "manul_snow", "manul_dune"];
let welcomeCycleTimer = 0;
let welcomeCycleIndex = 0;

function stopWelcomeCycle() {
  if (welcomeCycleTimer) {
    clearInterval(welcomeCycleTimer);
    welcomeCycleTimer = 0;
  }
}

function renderWelcomeGreetWord(word) {
  const host = $("#welcomeGreetCycle");
  if (!host) return;
  host.innerHTML = `<span class="welcome-greet-word">${esc(word)}</span>`;
}

function startWelcomeCycle() {
  stopWelcomeCycle();
  const startMap = { ru: 0, en: 1, uk: 2 };
  welcomeCycleIndex = startMap[CUR_LANG] ?? 0;
  renderWelcomeGreetWord(WELCOME_GREETS[welcomeCycleIndex]);
  if (window.K1ttyAppearance?.isReducedMotion?.()) return;
  welcomeCycleTimer = setInterval(() => {
    const modal = $("#welcomeModal");
    if (!modal || !modal.classList.contains("open")) {
      stopWelcomeCycle();
      return;
    }
    if ($("#welcomeStage1")?.hidden) return;
    welcomeCycleIndex = (welcomeCycleIndex + 1) % WELCOME_GREETS.length;
    renderWelcomeGreetWord(WELCOME_GREETS[welcomeCycleIndex]);
  }, 2100);
}

function syncWelcomeBadge(theme) {
  const badge = $("#welcomeCatBadge");
  if (!badge) return;
  const svg = window.K1ttyAppearance?.renderCatSvg?.(theme, "cat-art-welcome");
  if (svg) {
    badge.innerHTML = svg;
  } else {
    badge.innerHTML = `<svg class="welcome-brand-fallback" aria-hidden="true"><use href="#i-brand"/></svg>`;
  }
}

function renderWelcomeThemes() {
  const grid = $("#welcomeThemeGrid");
  if (!grid || !window.K1ttyAppearance) return;
  const active = window.K1ttyAppearance.getSettings().theme;
  grid.innerHTML = WELCOME_THEMES.map((name) => {
    const label = window.K1ttyAppearance.getThemeLabel(name, CUR_LANG);
    const catSvg = window.K1ttyAppearance.renderCatSvg(name, "cat-art-tile");
    const catMarkup = catSvg ? `<span class="appearance-theme-cat">${catSvg}</span>` : "";
    const isCat = !!catSvg;
    return `<button type="button" class="welcome-theme-card${isCat ? " is-cat-theme" : ""}" data-welcome-theme="${name}" role="radio" aria-checked="${active === name ? "true" : "false"}">
      <span class="appearance-theme-preview welcome-theme-thumb appearance-theme-${name}" aria-hidden="true">
        <span class="appearance-mini-sidebar"><i></i><i></i><i></i></span>
        <span class="appearance-mini-content"><i></i><i></i></span>
        ${catMarkup}
      </span>
      <span class="welcome-theme-label">${esc(label)}</span>
    </button>`;
  }).join("");
  syncWelcomeBadge(active);
}

function syncWelcomeControls() {
  $$("[data-welcome-lang]").forEach((btn) => {
    const active = btn.dataset.welcomeLang === CUR_LANG;
    btn.setAttribute("aria-checked", active ? "true" : "false");
  });
  renderWelcomeThemes();
}

function setWelcomeStep(step) {
  const sheet = $(".welcome-sheet");
  const s1 = $("#welcomeStage1");
  const s2 = $("#welcomeStage2");
  if (!sheet || !s1 || !s2) return;
  const isStep2 = step === 2;
  sheet.dataset.welcomeStep = isStep2 ? "2" : "1";
  s1.hidden = isStep2;
  s2.hidden = !isStep2;
  s1.classList.toggle("active", !isStep2);
  s2.classList.toggle("active", isStep2);
  $$("[data-welcome-dot]").forEach((dot) => {
    dot.classList.toggle("active", dot.dataset.welcomeDot === (isStep2 ? "2" : "1"));
  });
  if (isStep2) {
    renderWelcomeThemes();
    $("#welcomeStartBtn")?.focus({ preventScroll: true });
  } else {
    $("#welcomeNextBtn")?.focus({ preventScroll: true });
  }
}

function openWelcome() {
  const modal = $("#welcomeModal");
  if (!modal) return;
  syncWelcomeControls();
  setWelcomeStep(1);
  modal.classList.remove("open");
  void modal.offsetWidth;
  modal.classList.add("open");
  startWelcomeCycle();
}

function dismissWelcome() {
  closeModal("welcomeModal");
}

$("#welcomeNextBtn")?.addEventListener("click", () => setWelcomeStep(2));
$("#welcomeBackBtn")?.addEventListener("click", () => setWelcomeStep(1));
$("#welcomeSkipBtn")?.addEventListener("click", dismissWelcome);
$("#welcomeStartBtn")?.addEventListener("click", dismissWelcome);
$("#openWelcomeBtn")?.addEventListener("click", () => openWelcome());

$("#welcomeLangSeg")?.addEventListener("click", (event) => {
  const btn = event.target.closest("[data-welcome-lang]");
  if (!btn) return;
  setLang(btn.dataset.welcomeLang);
  startWelcomeCycle();
});

$("#welcomeThemeGrid")?.addEventListener("click", (event) => {
  const card = event.target.closest("[data-welcome-theme]");
  if (!card || !window.K1ttyAppearance) return;
  window.K1ttyAppearance.update({ theme: card.dataset.welcomeTheme });
  renderWelcomeThemes();
});

window.addEventListener("k1tty:appearancechange", () => {
  if ($("#welcomeModal")?.classList.contains("open")) {
    renderWelcomeThemes();
  }
});

let cutCtx = null;
async function openCut(key, mode) {
  if (!canMutateTranscript()) return;
  const { provider, id } = STATE.active;
  if (!guardNotRunning(provider)) return;
  const context = cutCtx = { provider, cid: id, key, mode, revision: null, writable: false, ready: false, pending: false };

  $("#cutTitle").textContent = mode === "head" ? t("cut_head_title") : t("cut_tail_title");
  $("#cutStats").innerHTML = `<div class="stat-box"><div class="skel-bar skel-w50"></div><span>${esc(t("calc"))}</span></div>`;
  $("#cutPreview").innerHTML = `<div class="skel-item"><div class="skel-bar skel-w90"></div><div class="skel-bar skel-w75"></div></div>`;

  const applyBtn = $("#cutApplyBtn");
  disarmButton(applyBtn);
  applyBtn.disabled = true;
  applyBtn.querySelector(".arm-label").textContent = t("delete");

  openModal("cutModal");
  startLoad();
  try {
    const r = await rpc("preview_cut", provider, { cid: id, key, mode });
    if (cutCtx !== context || !$("#cutModal").classList.contains("open")) return;
    context.revision = r.revision;
    context.writable = !!r.writable;
    context.ready = true;
    const rep = r.report;

    const stats = [];
    stats.push(`<div class="stat-box red"><b>${rep.steps_count ?? 0}</b><span>${esc(t("stats_items"))}</span></div>`);
    if (rep.raw_count != null) {
      stats.push(`<div class="stat-box red"><b>${rep.raw_count}</b><span>${esc(localizeRawLabel(rep.raw_label))}</span></div>`);
    }
    if (rep.bytes != null) {
      stats.push(`<div class="stat-box"><b>${fmtSize(rep.bytes)}</b><span>${esc(t("stats_size"))}</span></div>`);
    }
    $("#cutStats").innerHTML = stats.join("");

    const rows = (rep.steps_preview || []).map((p) =>
      `<div class="pl-row"><span class="pl-key">${esc(p.key)}</span><span class="pl-lbl">${esc(localizeKind(p.label))}</span><span class="pl-txt">${esc(p.text || "")}</span></div>`
    ).join("");
    $("#cutPreview").innerHTML = rows || `<div class="list-empty">${esc(t("no_preview"))}</div>`;
    applyBtn.disabled = !r.writable;
  } catch (e) {
    if (cutCtx === context && $("#cutModal").classList.contains("open")) {
      closeModal("cutModal");
      toast(e.message, "err");
    }
  } finally {
    stopLoad();
  }
}

$("#cutApplyBtn").onclick = () => {
  if (!cutCtx?.ready || !cutCtx.writable || cutCtx.pending) return;
  const context = cutCtx;
  const btn = $("#cutApplyBtn");
  handleArmedClick(btn, t("confirm"), async () => {
    if (cutCtx !== context || context.pending || !beginTranscriptMutation(context)) return;
    context.pending = true;
    btn.disabled = true;
    btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("deleting"))}</span>`;
    try {
      const r = await rpc("apply_cut", context.provider, {
        cid: context.cid,
        key: context.key,
        mode: context.mode,
        revision: context.revision,
        confirm: true,
      });
      const job = await waitJob(r.job_id);
      if (cutCtx === context) closeModal("cutModal");
      await finishTranscriptMutation(context, true);
      showReport(t("done"), job.result);
    } catch (e) {
      await finishTranscriptMutation(context, false);
      toast(e.message, "err");
    } finally {
      context.pending = false;
      if (cutCtx === context) {
        btn.disabled = !context.ready || !context.writable;
        btn.innerHTML = `${ico("trash")}<span class="arm-label">${esc(t("delete"))}</span><span class="arm-bar"></span>`;
      }
    }
  });
};

async function executeDeleteSingle(btn, key) {
  if (!canMutateTranscript()) return;
  const { provider, id } = STATE.active;
  const context = { provider, cid: id };
  if (!beginTranscriptMutation(context)) return;
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("deleting"))}</span>`;
  try {
    const r = await rpc("delete_single", provider, { cid: id, key, confirm: true });
    await waitJob(r.job_id);
    toast(t("toast_step_deleted"), "ok");
    if (btn.isConnected && STATE.active?.provider === provider && STATE.active.id === id && !STATE.active.readonlyBackup) {
      const anchor = captureTranscriptAnchor();
      const row = btn.closest(".msg-block, .tool-block, .notice-block");
      row?.remove();
      restoreTranscriptAnchor(anchor);
    }
    await finishTranscriptMutation(context, true);
    refreshAll(false);
  } catch (e) {
    await finishTranscriptMutation(context, false);
    toast(e.message, "err");
    btn.disabled = false;
    btn.innerHTML = `${ico("x")}<span class="arm-label">${esc(t("delete"))}</span><span class="arm-bar"></span>`;
  }
}

function showReport(title, report) {
  $("#repTitle").textContent = title;
  let html = "";
  const rep = (report && typeof report === "object") ? report : {};

  if (Array.isArray(rep.warns)) {
    for (const w of rep.warns) {
      if (w) html += `<div class="note warn">${ico("warn")}<span>${esc(w)}</span></div>`;
    }
  }

  if (Array.isArray(rep.leaks)) {
    if (rep.leaks.length) {
      html += `<div class="note err">${ico("warn")}<span>${esc(t("leaks_found", { n: rep.leaks.length }))}</span></div>`;
    } else {
      html += `<div class="note ok">${ico("search")}<span>${esc(t("leaks_none"))}</span></div>`;
    }
  }

  const rows = [];
  rows.push({
    key: t("rep_status"),
    html: `<span class="rep-badge ok">${ico("check")}<span>${esc(t("rep_ok"))}</span></span>`,
  });

  if (rep.integrity_check) {
    const ok = String(rep.integrity_check).toLowerCase() === "ok";
    rows.push({
      key: t("rep_integrity"),
      html: `<span class="rep-badge ${ok ? "ok" : "muted"}">${esc(String(rep.integrity_check).toUpperCase())}</span>`,
    });
  }

  const targetId = rep.session_id || rep.conversation_id || rep.tid || rep.deleted || "";
  if (targetId) {
    const shortId = String(targetId).length > 26 ? String(targetId).slice(0, 24) + "..." : String(targetId);
    rows.push({
      key: t("rep_dialog"),
      html: `<span title="${esc(targetId)}">${esc(shortId)}</span>`,
    });
  }

  if (rep.step_index != null) {
    rows.push({ key: t("rep_step"), html: `<span>#${esc(String(rep.step_index))}</span>` });
  } else if (rep.seq != null) {
    rows.push({ key: t("rep_step"), html: `<span>#${esc(String(rep.seq))}</span>` });
  }

  const removedCnt = rep.steps_deleted ?? rep.deleted_messages ?? rep.messages_deleted ?? rep.lines_removed ?? rep.removed_lines;
  if (removedCnt != null && Number(removedCnt) > 0) {
    rows.push({ key: t("rep_removed"), html: `<span>${esc(String(removedCnt))}</span>` });
  }
  if (rep.lines_patched != null && Number(rep.lines_patched) > 0) {
    rows.push({ key: t("rep_patched"), html: `<span>${esc(String(rep.lines_patched))}</span>` });
  }
  const turnsCnt = rep.turns_unlocked ?? rep.turns_locked;
  if (turnsCnt != null && Number(turnsCnt) > 0) {
    rows.push({ key: t("rep_turns"), html: `<span>${esc(String(turnsCnt))}</span>` });
  }
  if (rep.files_removed != null && Number(rep.files_removed) > 0) {
    rows.push({ key: t("rep_files_removed"), html: `<span>${esc(String(rep.files_removed))}</span>` });
  }
  const freedBytes = rep.bytes_freed ?? rep.bytes_removed ?? rep.freed_bytes;
  if (freedBytes != null && Number(freedBytes) > 0) {
    rows.push({ key: t("rep_freed"), html: `<span>${fmtSize(Number(freedBytes))}</span>` });
  }

  if (rep.backup) {
    const fullPath = String(rep.backup);
    const parts = fullPath.replace(/\\/g, "/").split("/").filter(Boolean);
    const shortName = parts[parts.length - 1] || fullPath;
    rows.push({
      key: t("rep_backup"),
      html: `<span title="${escAttr(fullPath)}">${esc(shortName)}</span><button type="button" class="rep-copy-btn" data-reveal-path="${escAttr(fullPath)}" title="${escAttr(t("reveal_in_explorer"))}" aria-label="${escAttr(t("reveal_in_explorer"))}">${ico("folder")}</button><button type="button" class="rep-copy-btn" data-copy-path="${escAttr(fullPath)}" title="${escAttr(t("rep_copy_path"))}" aria-label="${escAttr(t("rep_copy_path"))}">${ico("copy")}</button>`,
    });
  } else if ("backup" in rep) {
    rows.push({
      key: t("rep_backup"),
      html: `<span class="rep-badge muted">${esc(t("rep_backup_off"))}</span>`,
    });
  }

  html += `<div class="rep-summary"><div class="rep-card">${rows.map((r) => `
    <div class="rep-row">
      <span class="rep-key">${esc(r.key)}</span>
      <span class="rep-val">${r.html}</span>
    </div>`).join("")}</div></div>`;

  const body = $("#repBody");
  body.innerHTML = html;
  body.querySelectorAll("[data-copy-path]").forEach((btn) => {
    btn.onclick = async () => {
      try {
        await window.K1ttyDesktop.writeText(btn.dataset.copyPath || "");
        toast(t("rep_path_copied"), "ok");
      } catch {}
    };
  });
  openModal("repModal");
}

$("#repCloseBtn").onclick = async () => {
  closeModal("repModal");
  refreshAll(false);
};

let bkModalCtx = { provider: "all", cid: null, totalCount: 0, totalBytes: 0 };

function updateBkModalHeader() {
  const badge = $("#bkTotalBadge");
  const delAllBtn = $("#bkDeleteAllBtn");
  if (bkModalCtx.totalCount > 0) {
    badge.textContent = `${bkModalCtx.totalCount} · ${fmtSize(bkModalCtx.totalBytes)}`;
    badge.classList.remove("hidden");
    delAllBtn.classList.remove("hidden");
  } else {
    badge.classList.add("hidden");
    delAllBtn.classList.add("hidden");
  }
}

async function openBackups(provider = "all", cid = null) {
  bkModalCtx = { provider, cid, totalCount: 0, totalBytes: 0 };
  $("#bkTitle").textContent = cid ? t("dialog_backups") : t("backups");
  $("#bkTotalBadge").classList.add("hidden");
  $("#bkDeleteAllBtn").classList.add("hidden");
  $("#bkList").innerHTML = `<div class="skel-item"><div class="skel-bar skel-w75"></div><div class="skel-bar skel-w50"></div></div>`;
  openModal("bkModal");

  startLoad();
  try {
    const r = await rpc("backups", provider, { cid });
    const groups = r.groups || [];
    bkModalCtx.totalCount = Number(r.total_count) || 0;
    bkModalCtx.totalBytes = Number(r.total_bytes) || 0;
    updateBkModalHeader();

    if (!groups.length) {
      $("#bkList").innerHTML = `<div class="list-empty">${esc(t("no_backups"))}</div>`;
      return;
    }
    $("#bkList").innerHTML = groups.map((g) => {
      const gTitle = g.title || g.cid;
      const rows = g.backups.map((b) => `
        <div class="bk-row" data-bk-size="${Number(b.size) || 0}">
          <div class="bk-info">
            <span class="bk-name" title="${esc(b.dir)}">${esc(b.name)}</span>
            <span class="bk-size">${fmtSize(b.size || 0)}</span>
          </div>
          <div class="bk-actions">
            <button class="btn sm"
              data-act="inspect-bk"
              data-prov="${esc(b.provider)}"
              data-dir="${esc(b.dir)}"
              data-name="${esc(b.name)}"
              data-title="${esc(gTitle)}"
              title="${esc(t("inspect_bk"))}"
              ${b.inspectable === false ? "disabled" : ""}>
              ${ico("eye")}
              <span>${esc(t("inspect_bk"))}</span>
            </button>
            <button class="btn sm warn arm-btn"
              data-act="restore-bk"
              data-prov="${esc(b.provider)}"
              data-dir="${esc(b.dir)}"
              ${b.restorable ? "" : `disabled title="${esc(b.reason)}"`}>
              ${ico("restore")}
              <span class="arm-label">${esc(t("restore"))}</span>
              <span class="arm-bar"></span>
            </button>
            <button class="btn sm danger arm-btn"
              data-act="delete-bk"
              data-prov="${esc(b.provider)}"
              data-dir="${esc(b.dir)}"
              data-size="${Number(b.size) || 0}"
              title="${esc(t("delete"))}">
              ${ico("trash")}
              <span class="arm-label">${esc(t("delete"))}</span>
              <span class="arm-bar"></span>
            </button>
          </div>
        </div>`).join("");
      return `
        <div class="bk-group">
          <div class="bk-group-head">
            ${aiImg(g.provider)}
            <span class="bk-group-title">${esc(gTitle)}</span>
          </div>
          ${rows}
        </div>`;
    }).join("");
  } catch (e) {
    $("#bkList").innerHTML = `<div class="note err">${ico("alert")}<span>${esc(e.message)}</span></div>`;
  } finally {
    stopLoad();
  }
}

async function executeRestore(btn, provider, dir) {
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("restoring"))}</span>`;
  try {
    const r = await rpc("restore", provider, { backup_dir: dir, confirm: true });
    await waitJob(r.job_id);
    closeModal("bkModal");
    toast(t("toast_restored"), "ok");
    await refreshAll(true);
    if (STATE.active && !STATE.active.readonlyBackup) {
      await openThread(STATE.active.provider, STATE.active.id, { preserveSys: true });
    }
  } catch (e) {
    toast(e.message, "err");
    btn.disabled = false;
    btn.innerHTML = `${ico("restore")}<span class="arm-label">${esc(t("restore"))}</span><span class="arm-bar"></span>`;
  }
}

async function executeDeleteBackup(btn, provider, dir) {
  const row = btn.closest(".bk-row");
  const grp = btn.closest(".bk-group");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("deleting"))}</span>`;
  try {
    const r = await rpc("delete_backup", provider, { backup_dir: dir, confirm: true });
    const job = await waitJob(r.job_id);
    const freed = Number(job.result?.freed_bytes) || Number(btn.dataset.size) || 0;
    if (STATE.active && STATE.active.readonlyBackup && STATE.active.backupDir === dir) {
      showOverview();
    }
    row?.remove();
    if (grp && !grp.querySelector(".bk-row")) grp.remove();
    bkModalCtx.totalCount = Math.max(0, bkModalCtx.totalCount - 1);
    bkModalCtx.totalBytes = Math.max(0, bkModalCtx.totalBytes - freed);
    updateBkModalHeader();
    if (!$("#bkList .bk-row")) {
      $("#bkList").innerHTML = `<div class="list-empty">${esc(t("no_backups"))}</div>`;
    }
    toast(t("toast_bk_deleted", { size: fmtSize(freed) }), "ok");
    rpc("backups_summary", "all", {}).then((bs) => {
      STATE.backupsSummary = bs;
      if (!STATE.active) renderOverview();
    }).catch(() => {});
  } catch (e) {
    toast(e.message, "err");
    btn.disabled = false;
    btn.innerHTML = `${ico("trash")}<span class="arm-label">${esc(t("delete"))}</span><span class="arm-bar"></span>`;
  }
}

async function openDeleteAllBackups() {
  const listEl = $("#deleteAllBkList");
  const totalEl = $("#deleteAllBkTotal");
  const confirmBtn = $("#deleteAllBkConfirmBtn");
  listEl.innerHTML = `<div class="skel-item"><div class="skel-bar skel-w75"></div><div class="skel-bar skel-w50"></div></div>`;
  totalEl.innerHTML = "";
  confirmBtn.disabled = true;
  openModal("deleteAllBkModal");

  startLoad();
  try {
    const r = await rpc("backups_summary", "all", {});
    const rows = (r.providers || []).filter((p) => p.count > 0);
    if (!rows.length) {
      listEl.innerHTML = `<div class="list-empty">${esc(t("no_backups"))}</div>`;
      totalEl.innerHTML = "";
      confirmBtn.disabled = true;
      return;
    }
    listEl.innerHTML = rows.map((p) => `
      <div class="del-bk-row">
        ${aiImg(p.provider)}
        <span class="del-bk-prov">${esc(p.name)}</span>
        <span class="del-bk-cnt">${esc(t("bk_count_lbl", { n: p.count }))}</span>
        <span class="del-bk-sz">${fmtSize(p.bytes)}</span>
      </div>`).join("");
    totalEl.innerHTML = `
      <span>${esc(t("bk_total_lbl"))}</span>
      <span>${esc(t("bk_count_lbl", { n: r.total_count }))} · ${fmtSize(r.total_bytes)}</span>`;
    confirmBtn.disabled = false;
  } catch (e) {
    listEl.innerHTML = `<div class="note err">${ico("alert")}<span>${esc(e.message)}</span></div>`;
  } finally {
    stopLoad();
  }
}

async function executeDeleteAllBackups() {
  const btn = $("#deleteAllBkConfirmBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("deleting_all"))}</span>`;
  try {
    const r = await rpc("delete_all_backups", "all", { confirm: true });
    const job = await waitJob(r.job_id);
    const res = job.result || {};
    const deleted = Number(res.total_deleted) || 0;
    const freed = Number(res.freed_bytes) || 0;
    const breakdown = res.breakdown || [];

    closeModal("deleteAllBkModal");
    closeModal("bkModal");
    if (STATE.active && STATE.active.readonlyBackup) {
      showOverview();
    }

    $("#repTitle").textContent = t("bk_cleanup_done");
    const rowsHtml = breakdown.map((b) => `
      <div class="del-bk-row">
        ${aiImg(b.provider)}
        <span class="del-bk-prov">${esc(b.name)}</span>
        <span class="del-bk-cnt">${esc(t("bk_count_lbl", { n: b.deleted_count }))}</span>
        <span class="del-bk-sz">${fmtSize(b.freed_bytes)}</span>
      </div>`).join("");
    $("#repBody").innerHTML = `
      <div class="stat-grid">
        <div class="stat-box"><b>${deleted}</b><span>${esc(t("bk_deleted_count"))}</span></div>
        <div class="stat-box"><b>${fmtSize(freed)}</b><span>${esc(t("bk_freed_space"))}</span></div>
      </div>
      ${rowsHtml ? `<div class="del-bk-list">${rowsHtml}</div>` : ""}`;
    openModal("repModal");
    toast(t("toast_all_bk_deleted", { n: deleted, size: fmtSize(freed) }), "ok");
    rpc("backups_summary", "all", {}).then((bs) => {
      STATE.backupsSummary = bs;
      if (!STATE.active) renderOverview();
    }).catch(() => {});
  } catch (e) {
    toast(e.message, "err");
  } finally {
    btn.disabled = false;
    btn.innerHTML = `${ico("trash")}<span>${esc(t("delete_all_confirm"))}</span>`;
  }
}

$("#bkDeleteAllBtn").onclick = openDeleteAllBackups;
$("#deleteAllBkConfirmBtn").onclick = executeDeleteAllBackups;

$("#backupBackToListBtn").onclick = () => {
  openBackups(bkModalCtx.provider || "all", bkModalCtx.cid || null);
};
$("#backupExitBtn").onclick = () => {
  const prev = STATE.active?.prevLive;
  if (prev && STATE.allConvs.some((c) => c.provider === prev.provider && c.id === prev.id)) {
    openThread(prev.provider, prev.id, { preserveSys: false });
  } else {
    showOverview();
  }
};

function renderProviderPickGrid(container, selectedId) {
  container.innerHTML = PROV_ORDER.map((pid) => {
    const p = PROV[pid];
    const act = pid === selectedId;
    return `
      <button type="button" class="prov-pick-btn${act ? " active" : ""}" data-pick-prov="${pid}">
        ${aiImg(pid)}
        <span class="prov-pick-name">${esc(p.name)}</span>
      </button>`;
  }).join("");
}

let exportCtx = null;

function openExportModal(provider, cid, title) {
  exportCtx = { provider, cid, title: title || cid, preset: "balanced", lastUrl: null };
  $("#exportChatPill").innerHTML = `${aiImg(provider)}<span class="chat-pill-title">${esc(exportCtx.title)}</span><span class="chat-pill-prov">${esc(PROV[provider]?.name || provider)}</span>`;
  $$(".preset-card", $("#exportPresetGrid")).forEach((b) => {
    b.classList.toggle("active", b.dataset.packPreset === "balanced");
  });
  $("#exportIncludeTools").checked = true;
  $("#exportResult").classList.add("hidden");
  $("#exportResult").innerHTML = "";
  const btn = $("#exportApplyBtn");
  btn.disabled = false;
  btn.innerHTML = `${ico("download")}<span>${esc(t("export_btn"))}</span>`;
  openModal("exportModal");
}

$("#exportPresetGrid").addEventListener("click", (ev) => {
  const card = ev.target.closest(".preset-card");
  if (!card || !exportCtx) return;
  exportCtx.preset = card.dataset.packPreset;
  $$(".preset-card", $("#exportPresetGrid")).forEach((b) => {
    b.classList.toggle("active", b === card);
  });
});

async function triggerDownload(url, filename) {
  const token = url.replace(/^k1tty-export:/, "");
  return window.K1ttyDesktop.saveExport(token);
}

$("#exportApplyBtn").onclick = async () => {
  if (!exportCtx) return;
  const btn = $("#exportApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("exporting"))}</span>`;
  startLoad();
  try {
    const data = await window.K1ttyDesktop.exportPack({
      provider: exportCtx.provider,
      cid: exportCtx.cid,
      preset: exportCtx.preset,
      include_tools: $("#exportIncludeTools").checked,
    });
    const st = data.stats || {};
    const dlUrl = `k1tty-export:${data.token}`;
    exportCtx.lastUrl = dlUrl;
    const resEl = $("#exportResult");
    resEl.innerHTML = `
      <div class="pack-result-head">
        ${ico("check-circle")}
        <span>${esc(t("export_ready"))}</span>
        <span class="spacer"></span>
        <a class="btn sm" href="${dlUrl}" download="${esc(data.filename)}">${ico("download")}<span>${esc(t("download_again"))}</span></a>
      </div>
      <div class="pack-result-stats">
        <span>${fmtSize(st.raw_bytes)} -> ${fmtSize(st.packed_bytes)}</span>
        <span class="pack-ratio">-${st.ratio_pct ?? 0}%</span>
        <span>${st.entries ?? 0} ${esc(t("pack_entries"))}</span>
        <span>${st.elapsed_ms ?? 0} ${esc(t("pack_time"))}</span>
      </div>`;
    resEl.classList.remove("hidden");
    await triggerDownload(dlUrl, data.filename);
  } catch (e) {
    toast(e.message, "err");
  } finally {
    stopLoad();
    btn.disabled = false;
    btn.innerHTML = `${ico("download")}<span>${esc(t("export_btn"))}</span>`;
  }
};

let convertCtx = null;

function openConvertModal(provider, cid, title) {
  const defaultDst = PROV_ORDER.find((p) => p !== provider && STATE.providers[p]?.available)
    || PROV_ORDER.find((p) => p !== provider)
    || provider;
  convertCtx = { provider, cid, title: title || cid, dst: defaultDst };
  $("#convertChatPill").innerHTML = `${aiImg(provider)}<span class="chat-pill-title">${esc(convertCtx.title)}</span><span class="chat-pill-prov">${esc(PROV[provider]?.name || provider)}</span>`;
  renderProviderPickGrid($("#convertTargetGrid"), defaultDst);
  $("#convertTitleInput").value = convertCtx.title;
  $("#convertIncludeTools").checked = false;
  const btn = $("#convertApplyBtn");
  btn.disabled = false;
  btn.innerHTML = `${ico("repeat")}<span>${esc(t("convert_btn"))}</span>`;
  openModal("convertModal");
}

$("#convertTargetGrid").addEventListener("click", (ev) => {
  const b = ev.target.closest("[data-pick-prov]");
  if (!b || !convertCtx) return;
  convertCtx.dst = b.dataset.pickProv;
  $$("[data-pick-prov]", $("#convertTargetGrid")).forEach((x) => {
    x.classList.toggle("active", x === b);
  });
});

$("#convertApplyBtn").onclick = async () => {
  if (!convertCtx) return;
  if (!guardNotRunning(convertCtx.dst)) return;
  const btn = $("#convertApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("converting"))}</span>`;
  try {
    const r = await rpc("convert_chat", convertCtx.provider, {
      cid: convertCtx.cid,
      dst_provider: convertCtx.dst,
      title: $("#convertTitleInput").value.trim(),
      include_tools: $("#convertIncludeTools").checked,
    });
    const job = await waitJob(r.job_id);
    const res = job.result || {};
    closeModal("convertModal");
    toast(t("toast_converted", { prov: PROV[res.provider]?.name || convertCtx.dst }), "ok");
    await refreshAll(true);
    if (res.provider && res.cid) {
      await openThread(res.provider, res.cid, { preserveSys: false });
    }
  } catch (e) {
    toast(e.message, "err");
  } finally {
    btn.disabled = false;
    btn.innerHTML = `${ico("repeat")}<span>${esc(t("convert_btn"))}</span>`;
  }
};

let importCtx = null;

function showImportError(error) {
  const message = error ? String(error.message || error) : "";
  $("#importErrorDetails").textContent = message;
  $("#importError").classList.toggle("hidden", !message);
}

function setImportPending(context, pending) {
  context.pending = pending;
  if (context !== importCtx) return;
  for (const input of $$("#importTargetGrid button, #importDropZone, #importFileInput, #importTitleInput, #importIncludeTools")) {
    input.disabled = pending;
  }
  const btn = $("#importApplyBtn");
  btn.disabled = pending || !context.bytes;
  btn.setAttribute("aria-busy", String(pending));
  btn.innerHTML = pending
    ? `${ico("loader", "spin")}<span>${esc(t("importing"))}</span>`
    : `${ico("upload")}<span>${esc(t("import_btn"))}</span>`;
}

function openImportModal() {
  const defaultDst = (STATE.active && !STATE.active.readonlyBackup ? STATE.active.provider : null)
    || PROV_ORDER.find((p) => STATE.providers[p]?.available)
    || "antigravity";
  importCtx = { file: null, bytes: null, info: null, dst: defaultDst, request: 0, pending: false };
  $("#importFileInput").value = "";
  $("#importDropTitle").textContent = t("drop_k1tty_hint");
  $("#importDropMeta").textContent = "";
  $("#importInspectCard").classList.add("hidden");
  $("#importInspectCard").innerHTML = "";
  renderProviderPickGrid($("#importTargetGrid"), defaultDst);
  $("#importTitleInput").value = "";
  $("#importIncludeTools").checked = true;
  showImportError(null);
  setImportPending(importCtx, false);
  openModal("importModal");
}

async function handleImportFile(file) {
  if (!file || !importCtx || importCtx.pending) return;
  const context = importCtx;
  const request = ++context.request;
  const current = () => importCtx === context && context.request === request;
  context.file = file;
  context.bytes = null;
  context.info = null;
  showImportError(null);
  $("#importInspectCard").classList.add("hidden");
  $("#importDropTitle").textContent = file.name;
  $("#importDropMeta").textContent = `${fmtSize(file.size)} · ${t("inspecting")}`;
  $("#importApplyBtn").disabled = true;
  startLoad();
  try {
    const buf = await file.arrayBuffer();
    if (!current()) return;
    const data = await window.K1ttyDesktop.inspectPack(buf);
    if (!current()) return;
    context.bytes = data.token;
    const info = data.info;
    context.info = info;
    $("#importDropMeta").textContent = `${fmtSize(file.size)} · .k1tty v${info.version}`;
    const srcProv = info.source_provider;
    if (srcProv && PROV[srcProv]) {
      context.dst = srcProv;
      renderProviderPickGrid($("#importTargetGrid"), srcProv);
    }
    $("#importTitleInput").value = info.title || file.name.replace(/\.k1tty$/i, "");
    $("#importIncludeTools").checked = info.include_tools !== false;

    const card = $("#importInspectCard");
    card.innerHTML = `
      <div class="pack-result-head">
        ${srcProv && PROV[srcProv] ? aiImg(srcProv) : ico("package")}
        <span class="chat-pill-title">${esc(info.title || file.name)}</span>
        <span class="mono-tag">${esc(info.preset)}</span>
      </div>
      <div class="pack-result-stats">
        <span>${info.entry_count} ${esc(t("pack_entries"))}</span>
        <span>${fmtSize(info.packed_bytes)} / ${fmtSize(info.raw_bytes)}</span>
        <span class="pack-ratio">-${info.ratio_pct}%</span>
      </div>`;
    card.classList.remove("hidden");
    $("#importApplyBtn").disabled = false;
  } catch (e) {
    if (!current()) return;
    context.bytes = null;
    context.info = null;
    $("#importDropMeta").textContent = "";
    $("#importInspectCard").classList.add("hidden");
    showImportError(e);
  } finally {
    stopLoad();
  }
}

$("#importDropZone").onclick = () => $("#importFileInput").click();
$("#importFileInput").addEventListener("change", (ev) => {
  const f = ev.target.files?.[0];
  if (f) handleImportFile(f);
});
$("#importDropZone").addEventListener("dragover", (ev) => {
  ev.preventDefault();
  $("#importDropZone").classList.add("drag-over");
});
$("#importDropZone").addEventListener("dragleave", () => {
  $("#importDropZone").classList.remove("drag-over");
});
$("#importDropZone").addEventListener("drop", (ev) => {
  ev.preventDefault();
  $("#importDropZone").classList.remove("drag-over");
  const f = ev.dataTransfer?.files?.[0];
  if (f) handleImportFile(f);
});

$("#importTargetGrid").addEventListener("click", (ev) => {
  const b = ev.target.closest("[data-pick-prov]");
  if (!b || !importCtx || importCtx.pending) return;
  importCtx.dst = b.dataset.pickProv;
  showImportError(null);
  $$("[data-pick-prov]", $("#importTargetGrid")).forEach((x) => {
    x.classList.toggle("active", x === b);
  });
});

$("#importApplyBtn").onclick = async () => {
  if (!importCtx || !importCtx.bytes || importCtx.pending) return;
  const context = importCtx;
  if (!guardNotRunning(context.dst)) return;
  showImportError(null);
  setImportPending(context, true);
  let res;
  startLoad();
  try {
    const data = await window.K1ttyDesktop.importPack(
      context.bytes,
      context.dst,
      $("#importTitleInput").value.trim(),
      $("#importIncludeTools").checked,
    );
    res = data.result || {};
    context.bytes = null;
  } catch (e) {
    if (importCtx === context) showImportError(e);
    else toast(e.message || String(e), "err");
  } finally {
    stopLoad();
    setImportPending(context, false);
  }
  if (!res) return;
  if (importCtx === context) closeModal("importModal");
  toast(t("toast_imported", { prov: PROV[res.provider]?.name || context.dst }), "ok");
  try {
    await refreshAll(true);
    if (importCtx === context && res.provider && res.cid) {
      await openThread(res.provider, res.cid, { preserveSys: false });
    }
  } catch (e) {
    toast(e.message || String(e), "err");
  }
};

$("#providerTabs").addEventListener("click", (ev) => {
  const b = ev.target.closest(".seg-btn");
  if (!b) return;
  selectProviderTab(b.dataset.prov);
});

$("#searchInput").addEventListener("input", (ev) => {
  STATE.searchQuery = ev.target.value;
  const has = !!STATE.searchQuery;
  $("#searchClear").classList.toggle("hidden", !has);
  $("#searchKbd")?.classList.toggle("hidden", has);
  if (fullTextMode) scheduleGlobalSearch();
  renderSidebar();
});
$("#searchClear").onclick = () => {
  $("#searchInput").value = "";
  STATE.searchQuery = "";
  fullTextResults = [];
  fullTextLoading = false;
  $("#searchClear").classList.add("hidden");
  $("#searchKbd")?.classList.remove("hidden");
  renderSidebar();
  $("#searchInput").focus();
};

$("#convList").addEventListener("click", (ev) => {
  const more = ev.target.closest("[data-more]");
  const item = ev.target.closest(".conv-item");
  if (!item) return;
  if (more) {
    ev.stopPropagation();
    const rect = more.getBoundingClientRect();
    openCtxMenu(rect.right + 4, rect.bottom + 4, item.dataset.prov, item.dataset.cid, item);
    return;
  }
  const prov = item.dataset.prov;
  const cid = item.dataset.cid;
  if (multiSelectMode || ev.ctrlKey || ev.metaKey || ev.shiftKey) {
    if (!multiSelectMode) setMultiSelect(true);
    toggleSelectConv(prov, cid, ev.shiftKey);
    return;
  }
  const hitKey = item.dataset.hitKey || "";
  const hitQuery = fullTextMode ? STATE.searchQuery.trim() : "";
  openThread(prov, cid, { preserveSys: false }).then(() => {
    if (hitKey) {
      if (hitQuery) openThreadSearch(hitQuery);
      jumpToMessageKey(hitKey);
    }
  });
});

$("#convList").addEventListener("keydown", (ev) => {
  if (ev.key !== "Enter" && ev.key !== " ") return;
  const item = ev.target.closest(".conv-item");
  if (!item || ev.target.closest("[data-more]")) return;
  ev.preventDefault();
  if (multiSelectMode) {
    toggleSelectConv(item.dataset.prov, item.dataset.cid, ev.shiftKey);
    return;
  }
  openThread(item.dataset.prov, item.dataset.cid, { preserveSys: false });
});

$("#provGrid").addEventListener("click", (ev) => {
  if (ev.target.closest("[data-reveal-path]")) return;
  const provRow = ev.target.closest("[data-filter-prov]");
  if (provRow && selectionTouches(provRow)) return;
  const row = ev.target.closest("[data-filter-prov]");
  if (!row) return;
  selectProviderTab(row.dataset.filterProv);
  if (isMobile()) toggleDrawer(true);
});

$("#ovRecent").addEventListener("click", (ev) => {
  const row = ev.target.closest("[data-open-prov]");
  if (!row) return;
  openThread(row.dataset.openProv, row.dataset.openCid, { preserveSys: false });
});

$("#toggleSysBtn").onclick = async () => {
  if (!STATE.active?.hasRendered || transcriptMutationPending()) return;
  STATE.showSys = !STATE.showSys;
  syncSysButton();
  await refreshCurrentTranscript({ filterOnly: true });
};

$("#threadExportBtn").onclick = () => {
  if (STATE.active && !STATE.active.readonlyBackup) {
    openExportModal(STATE.active.provider, STATE.active.id, STATE.active.title);
  }
};
$("#threadConvertBtn").onclick = () => {
  if (STATE.active && !STATE.active.readonlyBackup) {
    openConvertModal(STATE.active.provider, STATE.active.id, STATE.active.title);
  }
};
$("#threadReloadBtn").onclick = () => {
  if (STATE.active && !STATE.active.readonlyBackup) {
    openThread(STATE.active.provider, STATE.active.id, { preserveSys: true });
  }
};
$("#threadBackupsBtn").onclick = () => {
  if (STATE.active && !STATE.active.readonlyBackup) {
    openBackups(STATE.active.provider, STATE.active.id);
  }
};
$("#allBackupsBtn").onclick = () => openBackups("all", null);
$("#refreshAllBtn").onclick = () => refreshAll(true);

document.addEventListener("click", (ev) => {
  const revealEl = ev.target.closest("[data-reveal-path]");
  if (revealEl) {
    ev.preventDefault();
    if (selectionTouches(revealEl)) return;
    ev.stopPropagation();
    revealLocalPath(revealEl.dataset.revealPath);
    return;
  }
  const closeBtn = ev.target.closest("[data-close]");
  if (closeBtn) { closeModal(closeBtn.dataset.close); return; }
  if (ev.target.classList.contains("modal-bg")) {
    ev.target.classList.remove("open");
    disarmAll();
    return;
  }

  const actEl = ev.target.closest("[data-act]");
  if (!actEl || actEl.disabled) return;
  const act = actEl.dataset.act;
  if (!canMutateTranscript() && ["edit", "cut-tail", "cut-head", "del-single"].includes(act)) return;

  if (act === "edit") {
    openEdit(actEl.dataset.key);
  } else if (act === "cut-tail") {
    openCut(actEl.dataset.key, "tail");
  } else if (act === "cut-head") {
    openCut(actEl.dataset.key, "head");
  } else if (act === "del-single") {
    if (!STATE.active || !guardNotRunning(STATE.active.provider)) return;
    handleArmedClick(actEl, t("confirm"), () => executeDeleteSingle(actEl, actEl.dataset.key));
  } else if (act === "inspect-bk") {
    openBackupResearch(actEl.dataset.prov, actEl.dataset.dir, actEl.dataset.name, actEl.dataset.title, { preserveSys: false });
  } else if (act === "restore-bk") {
    if (!guardNotRunning(actEl.dataset.prov)) return;
    handleArmedClick(actEl, t("confirm"), () => executeRestore(actEl, actEl.dataset.prov, actEl.dataset.dir));
  } else if (act === "delete-bk") {
    handleArmedClick(actEl, t("confirm"), () => executeDeleteBackup(actEl, actEl.dataset.prov, actEl.dataset.dir));
  } else if (act === "clamp") {
    const body = actEl.closest(".msg-block, .notice-block")?.querySelector(".msg-body");
    if (!body) return;
    const clamped = body.classList.toggle("clamped");
    actEl.innerHTML = clamped
      ? `${ico("chev-down")}<span>${esc(t("show_more"))}</span>`
      : `${ico("chev-up")}<span>${esc(t("collapse"))}</span>`;
  } else if (act === "copy-code") {
     
    const code = actEl.closest(".code-wrap")?.querySelector("pre")?.textContent
      ?? CODE_STORE[Number(actEl.dataset.codeIdx)] ?? "";
    window.K1ttyDesktop.writeText(code).then(() => {
      const orig = actEl.innerHTML;
      actEl.innerHTML = `${ico("check")}<span>${esc(t("copied"))}</span>`;
      setTimeout(() => { actEl.innerHTML = orig; }, 1400);
    });
  }
});

document.addEventListener("keydown", (ev) => {
  if (ev.key === "Escape") {
    closeCtxMenu();
    $$(".modal-bg.open").forEach((m) => m.classList.remove("open"));
    disarmAll();
    return;
  }
  if ((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "b") {
    ev.preventDefault();
    toggleDrawer();
    return;
  }
  if ((ev.ctrlKey || ev.metaKey) && ev.key === "Enter" && $("#editModal").classList.contains("open")) {
    ev.preventDefault();
    applyEdit();
    return;
  }
  if (ev.key === "/" && !["INPUT", "TEXTAREA"].includes(document.activeElement?.tagName)) {
    ev.preventDefault();
    if (!drawerDesktopOpen) toggleDrawer(true);
    if (isMobile()) toggleDrawer(true);
    $("#searchInput").focus();
  }
});

async function refreshAll(force = false) {
  const refBtn = $("#refreshAllBtn");
  refBtn.querySelector(".ico")?.classList.add("spin");
  if (!STATE.allConvs.length) $("#convList").innerHTML = convListSkeleton();
  startLoad();
  try {
    const [stRes, cvRes, bkRes] = await Promise.all([
      rpc("state", "all", { force }),
      rpc("convs", "all", { refresh: force }),
      rpc("backups_summary", "all", {}).catch(() => null),
    ]);
    STATE.providers = stRes.providers || {};
    if (stRes.settings) STATE.settings = stRes.settings;
    STATE.allConvs = cvRes.convs || [];
    if (bkRes) STATE.backupsSummary = bkRes;
    applyAutoBackupUI();
    renderSidebar();
    if (!STATE.active) renderOverview();
    refreshStatusBadge();
  } catch (e) {
    toast(e.message, "err");
  } finally {
    refBtn.querySelector(".ico")?.classList.remove("spin");
    stopLoad();
  }
}

function openFromHash() {
  const h = (location.hash || "").replace(/^#/, "");
  if (!h || !h.includes("/")) return false;
  const [prov, ...rest] = h.split("/");
  const cid = decodeURIComponent(rest.join("/"));
  if (PROV[prov] && cid) {
    openThread(prov, cid, { preserveSys: false });
    return true;
  }
  return false;
}

window.addEventListener("hashchange", () => {
  if (!openFromHash()) showOverview();
});

function attachOverlayScrollbar(scroller, host, { top = 8, bottom = 8, right = 4 } = {}) {
  if (!scroller || !host) return null;
  scroller.classList.add("lg-scroll-host");
  const track = document.createElement("div");
  track.className = "lg-sb-track";
  track.style.setProperty("--sb-top", top + "px");
  track.style.setProperty("--sb-bottom", bottom + "px");
  track.style.setProperty("--sb-right", right + "px");
  const thumb = document.createElement("div");
  thumb.className = "lg-sb-thumb";
  track.appendChild(thumb);
  host.appendChild(track);

  let hideTimer = null;
  let dragging = false;
  let dragStartY = 0;
  let dragStartScroll = 0;
  let rafId = 0;
  let lastThumbHeight = "";
  let lastThumbTransform = "";

  const pingVisible = () => {
    if (!track.classList.contains("has-scroll")) return;
    track.classList.add("visible");
    clearTimeout(hideTimer);
    if (!dragging) {
      hideTimer = setTimeout(() => track.classList.remove("visible"), 1100);
    }
  };

  const update = () => {
    rafId = 0;
    const isHidden = scroller.classList.contains("hidden") || scroller.offsetParent === null;
    const sh = scroller.scrollHeight;
    const ch = scroller.clientHeight;
    const th = track.clientHeight;
    if (isHidden || sh <= ch + 2 || th <= 0) {
      if (track.classList.contains("has-scroll") || track.classList.contains("visible")) track.classList.remove("has-scroll", "visible");
      return;
    }
    if (!track.classList.contains("has-scroll")) track.classList.add("has-scroll");
    const thumbH = Math.max(28, Math.round((ch / sh) * th));
    const maxScroll = Math.max(1, sh - ch);
    const maxY = Math.max(0, th - thumbH);
    const ratio = Math.min(1, Math.max(0, scroller.scrollTop / maxScroll));
    const height = thumbH + "px";
    const transform = `translateY(${(ratio * maxY).toFixed(1)}px)`;
    if (height !== lastThumbHeight) { thumb.style.height = height; lastThumbHeight = height; }
    if (transform !== lastThumbTransform) { thumb.style.transform = transform; lastThumbTransform = transform; }
  };

  const scheduleUpdate = () => {
    if (!rafId) rafId = requestAnimationFrame(update);
  };

  scroller.addEventListener("scroll", () => {
    scheduleUpdate();
    pingVisible();
  }, { passive: true });

  scroller.addEventListener("pointerenter", () => {
    scheduleUpdate();
    pingVisible();
  }, { passive: true });

  thumb.addEventListener("pointerdown", (ev) => {
    if (ev.button !== 0) return;
    ev.preventDefault();
    ev.stopPropagation();
    dragging = true;
    dragStartY = ev.clientY;
    dragStartScroll = scroller.scrollTop;
    track.classList.add("dragging", "visible");
    clearTimeout(hideTimer);
    thumb.setPointerCapture(ev.pointerId);
  });

  thumb.addEventListener("pointermove", (ev) => {
    if (!dragging) return;
    const sh = scroller.scrollHeight;
    const ch = scroller.clientHeight;
    const th = track.clientHeight;
    const thumbH = thumb.offsetHeight || 28;
    const maxY = Math.max(1, th - thumbH);
    const maxScroll = Math.max(1, sh - ch);
    const dy = ev.clientY - dragStartY;
    scroller.scrollTop = dragStartScroll + (dy / maxY) * maxScroll;
  });

  const endDrag = () => {
    if (!dragging) return;
    dragging = false;
    track.classList.remove("dragging");
    pingVisible();
  };
  thumb.addEventListener("pointerup", endDrag);
  thumb.addEventListener("pointercancel", endDrag);

  track.addEventListener("pointerdown", (ev) => {
    if (ev.target === thumb || ev.button !== 0) return;
    ev.preventDefault();
    const rect = track.getBoundingClientRect();
    const thumbH = thumb.offsetHeight || 28;
    const clickY = ev.clientY - rect.top - thumbH / 2;
    const maxY = Math.max(1, rect.height - thumbH);
    const ratio = Math.min(1, Math.max(0, clickY / maxY));
    scroller.scrollTo({ top: ratio * (scroller.scrollHeight - scroller.clientHeight), behavior: preferredScrollBehavior() });
    pingVisible();
  });

  if (typeof ResizeObserver !== "undefined") {
    const ro = new ResizeObserver(scheduleUpdate);
    ro.observe(scroller);
    if (scroller.firstElementChild) ro.observe(scroller.firstElementChild);
  }
  if (typeof MutationObserver !== "undefined") {
    const mo = new MutationObserver(scheduleUpdate);
    mo.observe(scroller, { childList: true, subtree: true, attributes: true, attributeFilter: ["class"] });
  }
  window.addEventListener("resize", scheduleUpdate, { passive: true });
  window.addEventListener("k1tty:appearancechange", scheduleUpdate);
  scheduleUpdate();
  return { update: scheduleUpdate, track };
}

function initCustomScrollbars() {
  attachOverlayScrollbar($("#feedScroll"), $("#chatWrap"), { top: 8, bottom: 8, right: 4 });
  attachOverlayScrollbar($("#overviewPane"), $(".main-body"), { top: 8, bottom: 8, right: 4 });
  attachOverlayScrollbar($("#convList"), $("#convListWrap"), { top: 4, bottom: 8, right: 2 });
}

const PINNED_KEY = "k1tty_pins_v1";
const pinnedSet = new Set();
try {
  const savedPins = JSON.parse(localStorage.getItem(PINNED_KEY) || "[]");
  if (Array.isArray(savedPins)) savedPins.forEach((k) => { if (typeof k === "string") pinnedSet.add(k); });
} catch {}

function savePins() {
  try { localStorage.setItem(PINNED_KEY, JSON.stringify([...pinnedSet])); } catch {}
}

function isPinned(prov, cid) {
  return pinnedSet.has(`${prov}:${cid}`);
}

function togglePin(prov, cid) {
  const key = `${prov}:${cid}`;
  const next = !pinnedSet.has(key);
  if (next) pinnedSet.add(key);
  else pinnedSet.delete(key);
  savePins();
  renderSidebar();
  toast(t(next ? "toast_pinned" : "toast_unpinned"), "ok");
}

let multiSelectMode = false;
const selectedConvKeys = new Set();
let lastSelectedConvKey = "";

function visibleConvKeys() {
  return $$("#convList .conv-item").map((el) => `${el.dataset.prov}:${el.dataset.cid}`);
}

function syncBatchBar() {
  const bar = $("#batchBar");
  const btn = $("#multiSelectBtn");
  if (!bar) return;
  bar.classList.toggle("hidden", !multiSelectMode);
  if (btn) {
    btn.setAttribute("aria-pressed", multiSelectMode ? "true" : "false");
    btn.classList.toggle("active-toggle", multiSelectMode);
  }
  if (!multiSelectMode) return;
  const count = selectedConvKeys.size;
  const countEl = $("#batchCount");
  if (countEl) countEl.textContent = t("selected_n", { n: count });
  const vis = visibleConvKeys();
  const allSelected = vis.length > 0 && vis.every((k) => selectedConvKeys.has(k));
  const selAllBtn = $("#batchSelectAllBtn");
  if (selAllBtn) selAllBtn.textContent = t(allSelected ? "deselect_all" : "select_all");
  const expBtn = $("#batchExportBtn");
  const delBtn = $("#batchDeleteBtn");
  if (expBtn) expBtn.disabled = count === 0;
  if (delBtn) {
    delBtn.disabled = count === 0;
    if (count === 0) disarmButton(delBtn);
  }
}

function setMultiSelect(on) {
  const next = !!on;
  if (multiSelectMode === next) {
    syncBatchBar();
    return;
  }
  multiSelectMode = next;
  if (!multiSelectMode) {
    selectedConvKeys.clear();
    lastSelectedConvKey = "";
    disarmButton($("#batchDeleteBtn"));
  }
  renderSidebar();
  syncBatchBar();
}

function toggleSelectConv(prov, cid, shiftRange = false) {
  const key = `${prov}:${cid}`;
  if (shiftRange && lastSelectedConvKey) {
    const vis = visibleConvKeys();
    const a = vis.indexOf(lastSelectedConvKey);
    const b = vis.indexOf(key);
    if (a >= 0 && b >= 0) {
      const [lo, hi] = a < b ? [a, b] : [b, a];
      for (let i = lo; i <= hi; i++) selectedConvKeys.add(vis[i]);
      lastSelectedConvKey = key;
      renderSidebar();
      return;
    }
  }
  if (selectedConvKeys.has(key)) selectedConvKeys.delete(key);
  else selectedConvKeys.add(key);
  lastSelectedConvKey = key;
  renderSidebar();
}

async function runBatchExport() {
  if (!selectedConvKeys.size) return;
  const items = [...selectedConvKeys].map((k) => {
    const idx = k.indexOf(":");
    return { provider: k.slice(0, idx), cid: k.slice(idx + 1) };
  });
  const btn = $("#batchExportBtn");
  if (btn) btn.disabled = true;
  startLoad();
  try {
    const res = await window.K1ttyDesktop.exportBatch(items, "balanced", true);
    if (res?.cancelled) return;
    const saved = Number(res?.saved) || 0;
    const failed = Array.isArray(res?.failed) ? res.failed.length : 0;
    if (saved > 0) toast(t("batch_exported", { n: saved }), "ok");
    if (failed > 0) toast(`${failed} error(s)`, "err");
    setMultiSelect(false);
  } catch (e) {
    toast(e.message || String(e), "err");
  } finally {
    stopLoad();
    syncBatchBar();
  }
}

async function runBatchDelete() {
  if (!selectedConvKeys.size) return;
  const items = [...selectedConvKeys].map((k) => {
    const idx = k.indexOf(":");
    return { provider: k.slice(0, idx), cid: k.slice(idx + 1), key: k };
  });
  startLoad();
  let deleted = 0;
  let skipped = 0;
  let activeRemoved = false;
  try {
    for (const item of items) {
      if (isConvLocked(item.provider, item.cid)) {
        skipped++;
        continue;
      }
      try {
        const r = await rpc("delete_conv", item.provider, { cid: item.cid, confirm: true });
        await waitJob(r.job_id);
        STATE.allConvs = STATE.allConvs.filter((c) => !(c.provider === item.provider && c.id === item.cid));
        selectedConvKeys.delete(item.key);
        pinnedSet.delete(item.key);
        if (STATE.active && STATE.active.provider === item.provider && STATE.active.id === item.cid) {
          activeRemoved = true;
        }
        deleted++;
      } catch {
        skipped++;
      }
    }
    savePins();
    if (activeRemoved) showOverview();
    setMultiSelect(false);
    renderSidebar();
    if (!STATE.active) renderOverview();
    if (deleted > 0) toast(t("batch_deleted", { n: deleted }), "ok");
    if (skipped > 0) toast(t("batch_skipped_locked", { n: skipped }), "err");
    rpc("backups_summary", "all", {}).then((bk) => {
      if (bk) {
        STATE.backupsSummary = bk;
        if (!STATE.active) renderOverview();
      }
    }).catch(() => {});
  } finally {
    stopLoad();
  }
}

$("#multiSelectBtn")?.addEventListener("click", () => setMultiSelect(!multiSelectMode));
$("#batchCancelBtn")?.addEventListener("click", () => setMultiSelect(false));
$("#batchSelectAllBtn")?.addEventListener("click", () => {
  const vis = visibleConvKeys();
  const allSelected = vis.length > 0 && vis.every((k) => selectedConvKeys.has(k));
  if (allSelected) vis.forEach((k) => selectedConvKeys.delete(k));
  else vis.forEach((k) => selectedConvKeys.add(k));
  renderSidebar();
});
$("#batchExportBtn")?.addEventListener("click", runBatchExport);
$("#batchDeleteBtn")?.addEventListener("click", (ev) => {
  ev.stopPropagation();
  if (!selectedConvKeys.size) return;
  handleArmedClick($("#batchDeleteBtn"), t("confirm_delete"), runBatchDelete);
});

let fullTextMode = false;
let fullTextResults = [];
let fullTextLoading = false;
let fullTextTimer = 0;
let fullTextSeq = 0;

function highlightSnippet(text, query) {
  const safe = esc(text || "");
  const q = (query || "").trim();
  if (!q) return safe;
  const escaped = q.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  try {
    return safe.replace(new RegExp(escaped, "gi"), (m) => `<mark>${m}</mark>`);
  } catch {
    return safe;
  }
}

function setFullTextMode(on) {
  fullTextMode = !!on;
  const btn = $("#searchFullTextBtn");
  if (btn) btn.setAttribute("aria-pressed", fullTextMode ? "true" : "false");
  if (fullTextMode) {
    scheduleGlobalSearch();
  } else {
    clearTimeout(fullTextTimer);
    fullTextLoading = false;
    fullTextResults = [];
  }
  renderSidebar();
}

function scheduleGlobalSearch() {
  clearTimeout(fullTextTimer);
  const q = STATE.searchQuery.trim();
  if (!fullTextMode || q.length < 2) {
    fullTextLoading = false;
    fullTextResults = [];
    renderSidebar();
    return;
  }
  fullTextLoading = true;
  renderSidebar();
  const seq = ++fullTextSeq;
  fullTextTimer = setTimeout(async () => {
    try {
      const res = await rpc("search_all", "all", { query: q, limit: 50 });
      if (seq !== fullTextSeq || !fullTextMode) return;
      fullTextResults = Array.isArray(res?.results) ? res.results : [];
    } catch (e) {
      if (seq !== fullTextSeq) return;
      fullTextResults = [];
      toast(e.message || String(e), "err");
    } finally {
      if (seq === fullTextSeq) {
        fullTextLoading = false;
        renderSidebar();
      }
    }
  }, 240);
}

$("#searchFullTextBtn")?.addEventListener("click", () => {
  setFullTextMode(!fullTextMode);
  $("#searchInput")?.focus();
});

let threadFindMatches = [];
let threadFindIndex = -1;
let threadFindTimer = 0;
let threadFindSeq = 0;

function clearThreadFindMarks() {
  $$("#chatFeed .msg-block.find-active").forEach((el) => el.classList.remove("find-active"));
}

function closeThreadSearch() {
  clearTimeout(threadFindTimer);
  threadFindMatches = [];
  threadFindIndex = -1;
  clearThreadFindMarks();
  const bar = $("#threadSearchBar");
  if (bar) bar.classList.add("hidden");
  const count = $("#threadSearchCount");
  if (count) count.textContent = "";
}

function openThreadSearch(prefill = "") {
  if (!STATE.active || STATE.active.readonlyBackup) return;
  const bar = $("#threadSearchBar");
  const input = $("#threadSearchInput");
  if (!bar || !input) return;
  bar.classList.remove("hidden");
  if (prefill) input.value = prefill;
  input.focus({ preventScroll: true });
  input.select();
  if (input.value.trim()) scheduleThreadSearch();
}

async function jumpToMessageKey(key) {
  if (!key || !STATE.active) return;
  const findEl = () => $(`#chatFeed [data-key="${CSS.escape(key)}"]`);
  let el = findEl();
  if (!el && !STATE.showSys) {
    STATE.showSys = true;
    syncSysButton();
    await refreshCurrentTranscript();
    el = findEl();
  }
  let guard = 0;
  while (!el && STATE.active?.hasMore && guard < 12) {
    guard++;
    const loadBtn = $("#moreWrap button");
    if (!loadBtn) break;
    loadBtn.click();
    await new Promise((r) => setTimeout(r, 140));
    let wait = 0;
    while (STATE.active?.paging && wait < 40) {
      wait++;
      await new Promise((r) => setTimeout(r, 60));
    }
    el = findEl();
  }
  if (!el) return;
  clearThreadFindMarks();
  el.classList.add("find-active");
  el.querySelectorAll("details").forEach((d) => { d.open = true; });
  el.scrollIntoView({ behavior: preferredScrollBehavior(), block: "center" });
}

function syncThreadFindCount() {
  const el = $("#threadSearchCount");
  if (!el) return;
  const q = ($("#threadSearchInput")?.value || "").trim();
  if (!q) {
    el.textContent = "";
    return;
  }
  if (!threadFindMatches.length) {
    el.textContent = t("find_no_matches");
    return;
  }
  el.textContent = `${threadFindIndex + 1} / ${threadFindMatches.length}`;
}

function stepThreadMatch(delta) {
  if (!threadFindMatches.length) return;
  threadFindIndex = (threadFindIndex + delta + threadFindMatches.length) % threadFindMatches.length;
  syncThreadFindCount();
  jumpToMessageKey(threadFindMatches[threadFindIndex].key);
}

function scheduleThreadSearch() {
  clearTimeout(threadFindTimer);
  const q = ($("#threadSearchInput")?.value || "").trim();
  if (!q || !STATE.active || STATE.active.readonlyBackup) {
    threadFindMatches = [];
    threadFindIndex = -1;
    clearThreadFindMarks();
    syncThreadFindCount();
    return;
  }
  const seq = ++threadFindSeq;
  const { provider, id } = STATE.active;
  threadFindTimer = setTimeout(async () => {
    try {
      const res = await rpc("search_thread", provider, { cid: id, query: q });
      if (seq !== threadFindSeq) return;
      threadFindMatches = Array.isArray(res?.matches) ? res.matches : [];
      threadFindIndex = threadFindMatches.length ? 0 : -1;
      syncThreadFindCount();
      if (threadFindIndex >= 0) jumpToMessageKey(threadFindMatches[0].key);
      else clearThreadFindMarks();
    } catch {
      if (seq === threadFindSeq) {
        threadFindMatches = [];
        threadFindIndex = -1;
        syncThreadFindCount();
      }
    }
  }, 180);
}

$("#threadFindBtn")?.addEventListener("click", () => {
  const bar = $("#threadSearchBar");
  if (bar && !bar.classList.contains("hidden")) closeThreadSearch();
  else openThreadSearch();
});
$("#threadSearchClose")?.addEventListener("click", closeThreadSearch);
$("#threadSearchInput")?.addEventListener("input", scheduleThreadSearch);
$("#threadSearchInput")?.addEventListener("keydown", (ev) => {
  if (ev.key === "Escape") {
    ev.preventDefault();
    ev.stopPropagation();
    closeThreadSearch();
  } else if (ev.key === "Enter") {
    ev.preventDefault();
    stepThreadMatch(ev.shiftKey ? -1 : 1);
  }
});
$("#threadSearchPrev")?.addEventListener("click", () => stepThreadMatch(-1));
$("#threadSearchNext")?.addEventListener("click", () => stepThreadMatch(1));

let lastContextStats = null;
let slimCtx = null;

function fmtTokens(n) {
  const v = Number(n) || 0;
  if (v >= 1000000) return (v / 1000000).toFixed(2) + "M";
  if (v >= 1000) return (v / 1000).toFixed(v >= 100000 ? 0 : 1) + "K";
  return String(v);
}

function syncOptionalHeaderTools() {
  const prefs = window.K1ttyAppearance?.getSettings?.() || { showTokenCounter: true, showSlimTool: true };
  const live = !!(STATE.active && !STATE.active.readonlyBackup);
  const tokPill = $("#threadTokenPill");
  const slimBtn = $("#threadSlimBtn");
  const showTok = live && prefs.showTokenCounter !== false && !!lastContextStats;
  const showSlim = live && prefs.showSlimTool !== false;
  if (tokPill) tokPill.classList.toggle("hidden", !showTok);
  if (slimBtn) slimBtn.classList.toggle("hidden", !showSlim);
  syncTopThreadSep();
}

async function loadContextStats(provider, cid, token) {
  lastContextStats = null;
  const tokPill = $("#threadTokenPill");
  if (tokPill) {
    tokPill.classList.add("hidden");
    tokPill.classList.remove("is-warn", "is-heavy");
  }
  try {
    const st = await rpc("context_stats", provider, { cid });
    if (token !== STATE.loadingThreadToken || !STATE.active || STATE.active.id !== cid) return;
    lastContextStats = st;
    if (tokPill) {
      const total = Number(st.total_tokens) || 0;
      const txt = $("#threadTokenText");
      if (txt) txt.textContent = `~${fmtTokens(total)} ${t("tok_unit")}`;
      tokPill.classList.toggle("is-heavy", total >= 140000);
      tokPill.classList.toggle("is-warn", total >= 80000 && total < 140000);
      tokPill.title = `${t("slim_total_tok")}: ~${fmtTokens(total)} ${t("tok_unit")} · ${t("slim_think_tok")}: ~${fmtTokens(st.thinking_tokens)} · ${t("slim_tool_tok")}: ~${fmtTokens(st.tool_tokens)}`;
    }
    syncOptionalHeaderTools();
  } catch {
    if (token === STATE.loadingThreadToken) syncOptionalHeaderTools();
  }
}

async function refreshSlimPreview() {
  if (!slimCtx) return;
  const applyBtn = $("#slimApplyBtn");
  disarmButton(applyBtn);
  const removeThinking = $("#slimRemoveThinking").checked;
  const trimTools = $("#slimTrimTools").checked;
  try {
    const prev = await rpc("preview_slim", slimCtx.provider, {
      cid: slimCtx.cid,
      remove_thinking: removeThinking,
      trim_heavy_tools: trimTools,
      tool_threshold_kb: slimCtx.thresholdKb,
    });
    slimCtx.preview = prev;
    $("#slimThinkingMeta").textContent = `${prev.thinking_count} · ${fmtSize(prev.thinking_bytes)} (~${fmtTokens(prev.thinking_tokens)} ${t("tok_unit")})`;
    $("#slimToolsMeta").textContent = `${prev.heavy_tool_count} · ${fmtSize(prev.heavy_tool_bytes)} (~${fmtTokens(prev.heavy_tool_tokens)} ${t("tok_unit")})`;
    const sav = $("#slimSavings");
    if (!prev.can_slim) {
      $("#slimOptionsCard").classList.add("hidden");
      sav.textContent = t("slim_unsupported");
      applyBtn.disabled = true;
      return;
    }
    $("#slimOptionsCard").classList.remove("hidden");
    if (prev.total_affected > 0) {
      sav.textContent = t("slim_savings_txt", {
        n: prev.total_affected,
        size: fmtSize(prev.freed_bytes),
        tok: fmtTokens(prev.freed_tokens),
      });
      applyBtn.disabled = false;
    } else {
      sav.textContent = t("slim_nothing");
      applyBtn.disabled = true;
    }
  } catch (e) {
    $("#slimSavings").textContent = e.message || String(e);
    applyBtn.disabled = true;
  }
}

async function openSlimModal() {
  if (!STATE.active || STATE.active.readonlyBackup) return;
  const { provider, id } = STATE.active;
  slimCtx = { provider, cid: id, thresholdKb: 4, preview: null };
  $("#slimRemoveThinking").checked = true;
  $("#slimTrimTools").checked = true;
  $$("[data-slim-kb]", $("#slimThresholdChips")).forEach((b) => {
    b.classList.toggle("active", Number(b.dataset.slimKb) === 4);
  });
  $("#slimStatsGrid").innerHTML = `<div class="slim-stat"><b>...</b><span>${esc(t("calc"))}</span></div>`;
  $("#slimSavings").textContent = t("calc");
  $("#slimHeaviestWrap").classList.add("hidden");
  const applyBtn = $("#slimApplyBtn");
  disarmButton(applyBtn);
  applyBtn.disabled = true;
  openModal("slimModal");
  try {
    const st = lastContextStats?.cid === id ? lastContextStats : await rpc("context_stats", provider, { cid: id });
    lastContextStats = st;
    $("#slimStatsGrid").innerHTML = `
      <div class="slim-stat"><b>~${fmtTokens(st.total_tokens)}</b><span>${esc(t("slim_total_tok"))}</span></div>
      <div class="slim-stat"><b>~${fmtTokens(st.thinking_tokens)}</b><span>${esc(t("slim_think_tok"))} (${st.thinking_steps})</span></div>
      <div class="slim-stat"><b>~${fmtTokens(st.tool_tokens)}</b><span>${esc(t("slim_tool_tok"))} (${st.tool_steps})</span></div>`;
    const heaviest = Array.isArray(st.heaviest) ? st.heaviest : [];
    if (heaviest.length) {
      $("#slimHeaviestWrap").classList.remove("hidden");
      $("#slimHeaviestList").innerHTML = heaviest.map((h) => `
        <button type="button" class="slim-heavy-item" data-jump-key="${esc(h.key || "")}">
          <span class="slim-heavy-tok">~${fmtTokens(h.tokens)} ${esc(t("tok_unit"))}</span>
          <span class="slim-heavy-prev">${esc(h.label ? `${h.label}: ${h.preview}` : h.preview)}</span>
        </button>`).join("");
    }
    await refreshSlimPreview();
  } catch (e) {
    toast(e.message || String(e), "err");
  }
}

async function executeSlim() {
  if (!slimCtx) return;
  if (!guardNotRunning(slimCtx.provider)) return;
  const applyBtn = $("#slimApplyBtn");
  applyBtn.disabled = true;
  startLoad();
  try {
    const r = await rpc("apply_slim", slimCtx.provider, {
      cid: slimCtx.cid,
      remove_thinking: $("#slimRemoveThinking").checked,
      trim_heavy_tools: $("#slimTrimTools").checked,
      tool_threshold_kb: slimCtx.thresholdKb,
      confirm: true,
    });
    const job = await waitJob(r.job_id);
    const res = job.result || {};
    closeModal("slimModal");
    toast(t("slim_done", { size: fmtSize(res.freed_bytes || 0), tok: fmtTokens(res.freed_tokens || 0) }), "ok");
    if (STATE.active && STATE.active.id === slimCtx.cid) {
      await refreshCurrentTranscript();
      loadContextStats(slimCtx.provider, slimCtx.cid, STATE.loadingThreadToken);
    }
  } catch (e) {
    toast(e.message || String(e), "err");
  } finally {
    stopLoad();
    disarmButton(applyBtn);
  }
}

$("#threadTokenPill")?.addEventListener("click", openSlimModal);
$("#threadSlimBtn")?.addEventListener("click", openSlimModal);
$("#slimRemoveThinking")?.addEventListener("change", refreshSlimPreview);
$("#slimTrimTools")?.addEventListener("change", refreshSlimPreview);
$("#slimThresholdChips")?.addEventListener("click", (ev) => {
  const chip = ev.target.closest("[data-slim-kb]");
  if (!chip || !slimCtx) return;
  slimCtx.thresholdKb = Number(chip.dataset.slimKb) || 4;
  $$("[data-slim-kb]", $("#slimThresholdChips")).forEach((b) => b.classList.toggle("active", b === chip));
  refreshSlimPreview();
});
$("#slimHeaviestList")?.addEventListener("click", (ev) => {
  const row = ev.target.closest("[data-jump-key]");
  if (!row || !row.dataset.jumpKey) return;
  closeModal("slimModal");
  jumpToMessageKey(row.dataset.jumpKey);
});
$("#slimApplyBtn")?.addEventListener("click", (ev) => {
  ev.stopPropagation();
  if (!slimCtx || $("#slimApplyBtn").disabled) return;
  handleArmedClick($("#slimApplyBtn"), t("confirm"), executeSlim);
});

let cmdItems = [];
let cmdIndex = 0;

function closeCmdPalette() {
  closeModal("cmdPaletteModal");
}

function buildCmdEntries(rawQuery) {
  const q = (rawQuery || "").trim().toLowerCase();
  const out = [];
  const actions = [
    { group: "actions", icon: "sliders", title: t("palette_open_settings"), meta: "", run: () => window.K1ttyAppearance?.open() },
    { group: "actions", icon: "upload", title: t("import_k1tty"), meta: "", run: () => openImportModal() },
    { group: "actions", icon: "history", title: t("backups"), meta: "", run: () => openBackups("all", null) },
    { group: "actions", icon: "file-text", title: t("palette_toggle_fulltext"), meta: "Ctrl+Shift+F", run: () => { toggleDrawer(true); setFullTextMode(true); $("#searchInput")?.focus(); } },
    { group: "actions", icon: "check-square", title: t("palette_toggle_select"), meta: "", run: () => { toggleDrawer(true); setMultiSelect(!multiSelectMode); } },
    { group: "actions", icon: "eye", title: t("palette_toggle_privacy"), meta: "", run: () => setPrivacy(!PRIVACY_ON) },
  ];
  if (STATE.active && !STATE.active.readonlyBackup) {
    actions.unshift({ group: "actions", icon: "sparkles", title: t("palette_open_slim"), meta: "", run: () => openSlimModal() });
    actions.unshift({ group: "actions", icon: "search", title: t("find_in_chat_tip"), meta: "Ctrl+F", run: () => openThreadSearch() });
  }
  const matchedActions = actions.filter((a) => !q || a.title.toLowerCase().includes(q));
  if (matchedActions.length) out.push({ header: t("palette_actions"), items: matchedActions });

  const matchedChats = STATE.allConvs
    .filter((c) => !q || (c.title || "").toLowerCase().includes(q) || (c.subtitle || "").toLowerCase().includes(q))
    .slice(0, q ? 10 : 6)
    .map((c) => ({
      group: "chats",
      prov: c.provider,
      title: c.title || c.id,
      meta: fmtShortMtime(c.mtime),
      run: () => openThread(c.provider, c.id, { preserveSys: false }),
    }));
  if (matchedChats.length) out.push({ header: t("palette_chats"), items: matchedChats });

  const allThemes = window.K1ttyAppearance?.allThemes || [];
  const matchedThemes = allThemes
    .map((th) => ({ id: th, label: window.K1ttyAppearance.getThemeLabel(th, CUR_LANG) }))
    .filter((th) => !q || th.label.toLowerCase().includes(q) || th.id.includes(q))
    .slice(0, q ? 12 : 6)
    .map((th) => ({
      group: "themes",
      icon: "sparkles",
      title: th.label,
      meta: th.id,
      run: () => window.K1ttyAppearance?.update({ theme: th.id }),
    }));
  if (matchedThemes.length) out.push({ header: t("palette_themes"), items: matchedThemes });
  return out;
}

function renderCmdPalette() {
  const input = $("#cmdPaletteInput");
  const list = $("#cmdPaletteList");
  if (!list) return;
  const groups = buildCmdEntries(input?.value || "");
  cmdItems = [];
  if (!groups.length) {
    list.innerHTML = `<div class="cmd-empty">${esc(t("palette_empty"))}</div>`;
    cmdIndex = -1;
    return;
  }
  let html = "";
  for (const g of groups) {
    html += `<div class="cmd-group-lbl">${esc(g.header)}</div>`;
    for (const item of g.items) {
      const idx = cmdItems.length;
      cmdItems.push(item);
      const icoHtml = item.prov ? aiImg(item.prov) : ico(item.icon || "command");
      html += `<button type="button" class="cmd-item${idx === 0 ? " active" : ""}" data-cmd-idx="${idx}" role="option" aria-selected="${idx === 0 ? "true" : "false"}">
        <span class="cmd-item-ico">${icoHtml}</span>
        <span class="cmd-item-title">${esc(item.title)}</span>
        ${item.meta ? `<span class="cmd-item-meta">${esc(item.meta)}</span>` : ""}
      </button>`;
    }
  }
  cmdIndex = 0;
  list.innerHTML = html;
}

function setCmdActive(next) {
  if (!cmdItems.length) return;
  cmdIndex = (next + cmdItems.length) % cmdItems.length;
  $$("[data-cmd-idx]", $("#cmdPaletteList")).forEach((el) => {
    const on = Number(el.dataset.cmdIdx) === cmdIndex;
    el.classList.toggle("active", on);
    el.setAttribute("aria-selected", on ? "true" : "false");
    if (on) el.scrollIntoView({ block: "nearest" });
  });
}

function executeCmdItem(idx) {
  const item = cmdItems[idx];
  if (!item) return;
  closeCmdPalette();
  item.run();
}

function openCmdPalette() {
  const modal = $("#cmdPaletteModal");
  const input = $("#cmdPaletteInput");
  if (!modal || !input) return;
  input.value = "";
  renderCmdPalette();
  openModal("cmdPaletteModal");
  requestAnimationFrame(() => input.focus({ preventScroll: true }));
}

$("#cmdPaletteBtn")?.addEventListener("click", openCmdPalette);
$("#cmdPaletteInput")?.addEventListener("input", renderCmdPalette);
$("#cmdPaletteInput")?.addEventListener("keydown", (ev) => {
  if (ev.key === "ArrowDown") { ev.preventDefault(); setCmdActive(cmdIndex + 1); }
  else if (ev.key === "ArrowUp") { ev.preventDefault(); setCmdActive(cmdIndex - 1); }
  else if (ev.key === "Enter") { ev.preventDefault(); executeCmdItem(cmdIndex); }
  else if (ev.key === "Escape") { ev.preventDefault(); closeCmdPalette(); }
});
$("#cmdPaletteList")?.addEventListener("click", (ev) => {
  const btn = ev.target.closest("[data-cmd-idx]");
  if (!btn) return;
  executeCmdItem(Number(btn.dataset.cmdIdx));
});

window.addEventListener("k1tty:find", () => {
  if (STATE.active && !STATE.active.readonlyBackup) {
    openThreadSearch();
  } else {
    toggleDrawer(true);
    const s = $("#searchInput");
    s?.focus();
    s?.select();
  }
});

window.addEventListener("k1tty:globalfind", () => {
  toggleDrawer(true);
  setFullTextMode(true);
  const s = $("#searchInput");
  s?.focus();
  s?.select();
});

window.addEventListener("k1tty:palette", () => {
  const modal = $("#cmdPaletteModal");
  if (modal?.classList.contains("open")) closeCmdPalette();
  else openCmdPalette();
});

window.addEventListener("k1tty:appearancechange", () => {
  syncOptionalHeaderTools();
});

document.addEventListener("keydown", (ev) => {
  if ((ev.ctrlKey || ev.metaKey) && !ev.altKey && ev.code === "KeyK" && !document.querySelector("dialog[open]")) {
    ev.preventDefault();
    const modal = $("#cmdPaletteModal");
    if (modal?.classList.contains("open")) closeCmdPalette();
    else openCmdPalette();
  }
});

initCustomScrollbars();
applyI18n();

async function boot() {
  $("#convList").innerHTML = convListSkeleton();
  renderOverview();
  await refreshAll(false);
  openFromHash();
}

wsConnect();
