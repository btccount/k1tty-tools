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
    ov_backups_title: "Резервные копии и вес на диске",
    auto_backup: "Автобекап",
    auto_backup_tip: "Автоматически создавать резервную копию перед правкой и удалением",
    auto_on: "ВКЛ",
    auto_off: "ВЫКЛ",
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
    running_guard_banner: "Файл диалога занят {name} · Закройте этот диалог или приложение для редактирования",
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
    welcome_desc: "Редактируй и чисти чаты, снимай баны и откатывай историю в пару кликов",
    continue: "Продолжить",
    welcome_sign_prefix: "с любовью от",
    welcome_sign_names: "quesada & котик <3",
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
    ov_backups_title: "Резервні копії та вага на диску",
    auto_backup: "Автобекап",
    auto_backup_tip: "Автоматично створювати резервну копію перед правкою та видаленням",
    auto_on: "УВІМК",
    auto_off: "ВИМК",
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
    running_guard_banner: "Файл діалогу зайнятий {name} · Закрийте цей діалог або застосунок для редагування",
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
    welcome_desc: "Редагуй і чисти чати, знімай бани та відкочуй історію в пару кліків",
    continue: "Продовжити",
    welcome_sign_prefix: "з любов'ю від",
    welcome_sign_names: "quesada & котик <3",
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
    ov_backups_title: "Backups and disk breakdown",
    auto_backup: "Auto-backup",
    auto_backup_tip: "Automatically create a backup before editing or deleting",
    auto_on: "ON",
    auto_off: "OFF",
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
    running_guard_banner: "Dialogue file is in use by {name} · Close the dialogue or app to edit",
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
    welcome_desc: "Edit and clean chats, lift locks, and roll back history in a couple clicks",
    continue: "Continue",
    welcome_sign_prefix: "with love from",
    welcome_sign_names: "quesada & kitty <3",
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

function localizeTotalLabel(lbl) {
  if (!lbl || CUR_LANG === "ru") return lbl || "";
  if (CUR_LANG === "uk") {
    return String(lbl)
      .replace("ходов:", "ходів:")
      .replace("записей:", "записів:")
      .replace("загружено:", "завантажено:");
  }
  return String(lbl)
    .replace("ходов:", "turns:")
    .replace("записей:", "entries:")
    .replace("загружено:", "loaded:");
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

function applyAutoBackupUI() {
  const on = STATE.settings?.auto_backup !== false;
  for (const [inpId, wrapId, stateId] of [
    ["#ovAutoBackupToggle", "#ovAutoBackupWrap", "#ovAutoBackupState"],
    ["#bkAutoBackupToggle", "#bkAutoBackupWrap", "#bkAutoBackupState"],
  ]) {
    const inp = $(inpId);
    const wrap = $(wrapId);
    const st = $(stateId);
    if (inp) inp.checked = on;
    if (wrap) wrap.classList.toggle("is-on", on);
    if (st) st.textContent = on ? t("auto_on") : t("auto_off");
  }
}

async function setAutoBackup(next) {
  const val = !!next;
  STATE.settings = { ...(STATE.settings || {}), auto_backup: val };
  applyAutoBackupUI();
  if (!STATE.active) renderOverview();
  try {
    const r = await rpc("set_settings", "all", { auto_backup: val });
    if (r.settings) STATE.settings = r.settings;
    applyAutoBackupUI();
    if (!STATE.active) renderOverview();
    toast(t(val ? "toast_autobk_on" : "toast_autobk_off"), "ok");
  } catch (e) {
    toast(e.message, "err");
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
  $$(".lang-btn", $("#langSwitch")).forEach((b) => {
    b.classList.toggle("active", b.dataset.lang === CUR_LANG);
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
    if (STATE.active.totalLabel) {
      $("#threadMeta").textContent = localizeTotalLabel(STATE.active.totalLabel);
    }
    if (STATE.active.lastTurns) {
      $("#chatFeed").innerHTML = renderTurnsHTML(STATE.active.lastTurns, STATE.active.provider) ||
        `<div class="list-empty">${esc(t("no_messages"))}</div>`;
      renderMoreBar(!!STATE.active.hasMore);
    }
  }
  renderSidebar();
  if (!STATE.active) renderOverview();
  refreshStatusBadge();
}

function setLang(lang) {
  if (!I18N[lang]) return;
  CUR_LANG = lang;
  try { localStorage.setItem(LANG_KEY, lang); } catch {}
  applyI18n();
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

let WS = null;
let WS_SEQ = 0;
let WS_READY = false;
let WAS_CONNECTED = false;
const WS_PEND = new Map();
const jobHandlers = new Map();
const finishedJobs = new Map();

function wsConnect() {
  const proto = location.protocol === "https:" ? "wss://" : "ws://";
  WS = new WebSocket(proto + location.host + "/ws");

  WS.addEventListener("open", () => {
    WS_READY = true;
    if (!window.__booted) {
      window.__booted = true;
      WAS_CONNECTED = true;
      boot();
    } else {
      if (!WAS_CONNECTED) toast(t("toast_reconnected"), "ok");
      WAS_CONNECTED = true;
      refreshAll(false);
    }
  });

  WS.addEventListener("message", (ev) => {
    let m;
    try { m = JSON.parse(ev.data); } catch { return; }
    if (m.type === "job") return onJobEvent(m.job);
    if (m.type === "procs") return onProcsUpdate(m.providers);
    if (m.type === "conv_lock") return onConvLockUpdate(m);
    if (m.type === "settings" && m.settings) {
      STATE.settings = m.settings;
      applyAutoBackupUI();
      if (!STATE.active) renderOverview();
      return;
    }
    if (m.type === "hello") {
      const bid = String(m.boot_id || "");
      try {
        if (bid && sessionStorage.getItem("k1tty_boot_id") !== bid) {
          sessionStorage.setItem("k1tty_boot_id", bid);
          requestAnimationFrame(() => openWelcome());
        }
      } catch {
        requestAnimationFrame(() => openWelcome());
      }
      return;
    }
    const p = WS_PEND.get(m.id);
    if (!p) return;
    WS_PEND.delete(m.id);
    if (m.ok) p.resolve(m);
    else p.reject(new Error(m.error || t("err_server")));
  });

  WS.addEventListener("close", () => {
    WS_READY = false;
    if (WAS_CONNECTED) {
      WAS_CONNECTED = false;
      toast(t("toast_reconnecting"), "err");
    }
    for (const [, p] of WS_PEND) p.reject(new Error(t("err_conn_drop")));
    WS_PEND.clear();
    for (const [, p] of jobHandlers) p.reject(new Error(t("err_conn_lost")));
    jobHandlers.clear();
    setTimeout(wsConnect, 1100);
  });

  WS.addEventListener("error", () => {});
}

function rpc(op, provider = "all", data = {}) {
  return new Promise((resolve, reject) => {
    if (!WS_READY) {
      reject(new Error(t("err_no_conn")));
      return;
    }
    const id = "r" + (++WS_SEQ);
    WS_PEND.set(id, { resolve, reject });
    WS.send(JSON.stringify({ id, op, provider, data }));
    setTimeout(() => {
      if (WS_PEND.has(id)) {
        WS_PEND.delete(id);
        reject(new Error(t("err_timeout", { op })));
      }
    }, 120000);
  });
}

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
}

function fmtSize(bytes) {
  if (bytes == null || isNaN(bytes)) return "0 " + t("u_b");
  if (bytes < 1024) return Math.round(bytes) + " " + t("u_b");
  if (bytes < 1048576) return Math.round(bytes / 1024) + " " + t("u_kb");
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + " " + t("u_mb");
  return (bytes / 1073741824).toFixed(1) + " " + t("u_gb");
}

function fmtTime(s) {
  if (!s) return "";
  const d = new Date(s);
  if (isNaN(d)) return "";
  return d.toLocaleString(t("date_locale"), { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
}

const CUR_YEAR = String(new Date().getFullYear());
function fmtShortMtime(s) {
  if (!s) return "";
  return String(s).replace("." + CUR_YEAR + " ", " ");
}

const CODE_STORE = [];
function fmtMarkdown(raw) {
  if (!raw) return `<span class="empty">${esc(t("empty"))}</span>`;
  const blocks = [];
  let text = String(raw).replace(/```([a-zA-Z0-9_+-]*)\n?([\s\S]*?)```/g, (_, lang, code) => {
    const idx = CODE_STORE.length;
    const cleanCode = code.replace(/\n$/, "");
    CODE_STORE.push(cleanCode);
    const token = `\x00CB${blocks.length}\x00`;
    blocks.push(
      `<div class="code-wrap">` +
        `<div class="code-head">` +
          `<span>${esc(lang || "code")}</span>` +
          `<button class="btn sm" data-act="copy-code" data-code-idx="${idx}">${ico("copy")}<span>${esc(t("copy"))}</span></button>` +
        `</div>` +
        `<pre class="codeblock">${esc(cleanCode)}</pre>` +
      `</div>`
    );
    return token;
  });

  text = esc(text);
  text = text.replace(/`([^`\n]+)`/g, '<code class="inline">$1</code>');
  text = text.replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>");
  text = text.replace(/^### (.+)$/gm, "<h3>$1</h3>");
  text = text.replace(/^## (.+)$/gm, "<h2>$1</h2>");
  text = text.replace(/^# (.+)$/gm, "<h1>$1</h1>");

  blocks.forEach((html, i) => {
    text = text.replace(`\x00CB${i}\x00`, html);
  });
  return text;
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

function toggleDrawer(force) {
  const root = $("#appRoot");
  if (isMobile()) {
    const next = force !== undefined ? force : !root.classList.contains("drawer-mobile-open");
    root.classList.toggle("drawer-mobile-open", next);
  } else {
    const nextCollapsed = force !== undefined ? !force : !root.classList.contains("drawer-collapsed");
    root.classList.toggle("drawer-collapsed", nextCollapsed);
  }
}

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
$("#langSwitch").addEventListener("click", (ev) => {
  const b = ev.target.closest(".lang-btn");
  if (!b) return;
  setLang(b.dataset.lang);
});

function syncCatBtn() {
  const btn = $("#catToggleBtn");
  if (!btn || !window.K1ttyCat) return;
  btn.classList.toggle("cat-off", !window.K1ttyCat.isEnabled());
}
$("#catToggleBtn")?.addEventListener("click", () => {
  if (!window.K1ttyCat) return;
  window.K1ttyCat.setEnabled(!window.K1ttyCat.isEnabled());
  syncCatBtn();
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
  const q = STATE.searchQuery.trim().toLowerCase();
  const list = STATE.allConvs.filter((c) => {
    if (STATE.filterProv !== "all" && c.provider !== STATE.filterProv) return false;
    if (!q) return true;
    return (
      (c.title || "").toLowerCase().includes(q) ||
      (c.subtitle || "").toLowerCase().includes(q) ||
      (c.id || "").toLowerCase().includes(q)
    );
  });

  const counts = { all: STATE.allConvs.length };
  for (const pid of PROV_ORDER) counts[pid] = 0;
  for (const c of STATE.allConvs) counts[c.provider] = (counts[c.provider] || 0) + 1;
  for (const [k, v] of Object.entries(counts)) {
    const el = $("#cnt-" + k);
    if (el) el.textContent = v;
  }

  const box = $("#convList");
  if (!list.length) {
    box.innerHTML = `<div class="list-empty">${esc(t("no_dialogs"))}</div>`;
    return;
  }

  const activeKey = STATE.active ? `${STATE.active.provider}:${STATE.active.id}` : "";
  const actTip = esc(t("actions"));
  box.innerHTML = list.map((c) => {
    const isAct = `${c.provider}:${c.id}` === activeKey;
    return `
      <div class="conv-item${isAct ? " active" : ""}" data-prov="${esc(c.provider)}" data-cid="${esc(c.id)}" role="button" tabindex="0">
        <span class="conv-ico">${aiImg(c.provider)}</span>
        <div class="conv-info">
          <div class="conv-title">${esc(c.title)}</div>
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
    const r = STATE.providers[pid].running;
    if (Array.isArray(r) && r.length) {
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
  const autoOn = STATE.settings?.auto_backup !== false;

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
      <div class="ov-stat-top">
        <span class="ov-stat-label">${esc(t("stat_backups"))}</span>
        <span class="ov-stat-tag ${autoOn ? "on" : "off"}">${esc(t("auto_backup"))}: ${esc(autoOn ? t("auto_on") : t("auto_off"))}</span>
      </div>
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
    const st = STATE.providers[pid];
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
          <span class="prov-row-path" title="${esc(st.appdata)}">${esc(st.appdata)}</span>
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
      : t("stat_backups_empty");
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
    bkGridEl.innerHTML = order.map((pid) => {
      const info = bkByProv[pid];
      const sharePct = bkTotalBytes > 0 ? (info.bytes / bkTotalBytes) * 100 : 0;
      const pctTxt = info.bytes > 0 && sharePct < 1 ? "<1%" : `${Math.round(sharePct)}%`;
      const cntTxt = info.count > 0 ? t("bk_count_lbl", { n: info.count }) : t("bk_none_short");
      return `
        <button type="button" class="ov-bk-card${info.count ? "" : " is-empty"}" data-bk-prov="${pid}">
          <div class="ov-bk-card-top">
            ${aiImg(pid)}
            <span class="ov-bk-card-name">${PROV[pid].name}</span>
          </div>
          <div class="ov-bk-card-size">${fmtSize(info.bytes)}</div>
          <div class="ov-bk-meter">
            <div class="ov-bk-meter-fill prov-${pid}" style="width:${Math.min(100, info.bytes > 0 ? Math.max(4, sharePct) : 0).toFixed(1)}%"></div>
          </div>
          <div class="ov-bk-card-meta">
            <span>${esc(cntTxt)}</span>
            <span>${info.bytes > 0 ? pctTxt : "0%"}</span>
          </div>
        </button>`;
    }).join("");
  }
}

function showOverview() {
  STATE.active = null;
  if (WS_READY) rpc("set_view", "all", {}).catch(() => {});
  history.replaceState(null, "", "#");
  $("#overviewPane").classList.remove("hidden");
  $("#chatWrap").classList.add("hidden");
  $("#backupBanner").classList.add("hidden");
  $("#threadStatusBtn").classList.add("hidden");
  $("#threadLockBtn").classList.add("hidden");
  $("#threadProvBadge").classList.add("hidden");
  $("#threadMeta").classList.add("hidden");
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
  renderSidebar();
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
  const lockBtn = $("#ctxLockBtn");
  if (lockBtn) lockBtn.classList.toggle("hidden", prov !== "codex");
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
  const { provider, cid, el } = ctxTarget;

  if (action === "open") {
    closeCtxMenu();
    openThread(provider, cid, { preserveSys: false });
  } else if (action === "export") {
    closeCtxMenu();
    openExportModal(provider, cid, ctxTarget.title);
  } else if (action === "convert") {
    closeCtxMenu();
    openConvertModal(provider, cid, ctxTarget.title);
  } else if (action === "backups") {
    closeCtxMenu();
    openBackups(provider, cid);
  } else if (action === "copy-id") {
    closeCtxMenu();
    navigator.clipboard?.writeText(cid).then(() => toast(t("toast_id_copied"), "ok"));
  } else if (action === "lock-chat") {
    closeCtxMenu();
    if (!guardNotRunning(provider)) return;
    openLockModal(provider, cid);
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
    if (STATE.active && STATE.active.provider === provider && STATE.active.id === cid) {
      showOverview();
    } else {
      renderSidebar();
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
    const isErr = it.status === "error";
    const name = localizeKind(it.label || it.tool || t("tool"));
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
          <div class="tool-body">${esc(it.text || t("empty"))}</div>
          ${actionsBarHTML(it)}
        </details>
      </div>`;
  }

  const isErr = it.kind === "error";
  const isGuard = it.subtype === "safeguard" || (it.text || "").toLowerCase().includes("safeguard");
  const iconName = isErr ? "alert" : (isGuard ? "shield" : "info");
  const long = (it.text || "").length > 900;
  const noticeTitle = isErr ? localizeUnlockerCategory(localizeKind(it.label || t("error"))) : localizeKind(it.label || t("system"));
  return `
    <article class="notice-block${isErr ? " is-error" : ""}" data-key="${k}">
      <div class="msg-head">
        <span class="role-chip">${ico(iconName)}</span>
        <span class="msg-who">${esc(noticeTitle)}</span>
        ${it.code ? `<span class="mono-tag">${esc(it.code)}</span>` : ""}
        ${it.ts ? `<span class="msg-time">${fmtTime(it.ts)}</span>` : ""}
      </div>
      <div class="msg-body${long ? " clamped" : ""}">${esc(it.text || t("empty"))}</div>
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
    "#threadStatusBtn",
    "#threadLockBtn",
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

function syncRunningGuard() {
  const wrap = $("#chatWrap");
  const banner = $("#runningGuardBanner");
  const txtEl = $("#runningGuardText");
  if (!wrap || !banner || !txtEl) return;
  if (!STATE.active || STATE.active.readonlyBackup) {
    wrap.classList.remove("app-locked");
    banner.classList.add("hidden");
    return;
  }
  const pid = STATE.active.provider;
  const locked = isConvLocked(pid, STATE.active.id);
  wrap.classList.toggle("app-locked", locked);
  if (locked) {
    const pname = PROV[pid]?.name || pid;
    txtEl.textContent = t("running_guard_banner", { name: pname });
    banner.classList.remove("hidden");
  } else {
    banner.classList.add("hidden");
  }
  const lockBtn = $("#threadLockBtn");
  if (lockBtn) lockBtn.disabled = locked;
  const unlockBtn = $("#unlockerApplyBtn");
  if (unlockBtn) unlockBtn.disabled = locked;
}

function onConvLockUpdate(msg) {
  if (!STATE.active || STATE.active.readonlyBackup) return;
  if (STATE.active.provider !== msg.provider || STATE.active.id !== msg.cid) return;
  STATE.active.fileLocked = !!msg.locked;
  STATE.active.lockReason = msg.reason || "";
  refreshStatusBadge();
}

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
  if (activeFileLocked || st.blocked) {
    show = true;
    cls = "err";
    iconName = "lock";
    pillTxt = t("pill_blocked", { name: pname });
    msg = t("toast_close_app_first", { name: pname });
  } else if (st.running && st.running.length) {
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

async function openThread(provider, cid, { preserveSys = true } = {}) {
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
  $("#threadMeta").classList.add("hidden");

  $("#toggleSysBtn").classList.remove("hidden");
  $("#threadConvertBtn").classList.remove("hidden");
  $("#threadExportBtn").classList.remove("hidden");
  $("#threadBackupsBtn").classList.remove("hidden");
  $("#threadReloadBtn").classList.remove("hidden");
  $("#topThreadSep").classList.remove("hidden");
  $("#threadLockBtn").classList.toggle("hidden", provider !== "codex");
  hideUnlockerCard();
  syncSysButton();
  refreshStatusBadge();
  renderSidebar();

  $("#moreWrap").innerHTML = "";
  $("#chatFeed").innerHTML = feedSkeleton();
  CODE_STORE.length = 0;

  startLoad();
  try {
    const r = await rpc("entries", provider, { cid, limit: 80, include_tools: STATE.showSys });
    if (token !== STATE.loadingThreadToken) return;
    STATE.cursor = r.cursor;
    STATE.loadedMsgs = r.page_msgs || 0;
    STATE.active.writable = r.writable !== false;
    STATE.active.fileLocked = !!r.file_locked;
    STATE.active.lockReason = r.lock_reason || "";
    STATE.active.blockedInfo = r.blocked_info || null;
    STATE.active.totalLabel = r.total_label || "";
    STATE.active.lastTurns = r.turns || [];
    STATE.active.hasMore = !!r.has_more;
    if (Array.isArray(r.block_presets) && r.block_presets.length) {
      LOCK_PRESETS = r.block_presets;
    }
    updateThreadLockBtn();
    refreshStatusBadge();

    const metaEl = $("#threadMeta");
    if (r.total_label) {
      metaEl.textContent = localizeTotalLabel(r.total_label);
      metaEl.classList.remove("hidden");
    } else {
      metaEl.classList.add("hidden");
    }

    $("#chatFeed").innerHTML = renderTurnsHTML(r.turns || [], provider) ||
      `<div class="list-empty">${esc(t("no_messages"))}</div>`;
    renderMoreBar(r.has_more);

    if (r.blocked_info && r.blocked_info.is_blocked) {
      showUnlockerCard(r.blocked_info);
    } else {
      hideUnlockerCard();
    }

    const sc = $("#feedScroll");
    requestAnimationFrame(() => { sc.scrollTop = sc.scrollHeight; updateScrollBtn(); });
  } catch (e) {
    if (token !== STATE.loadingThreadToken) return;
    $("#chatFeed").innerHTML = `<div class="note err">${ico("alert")}<span>${esc(e.message)}</span></div>`;
  } finally {
    stopLoad();
  }
}

async function openBackupResearch(provider, backupDir, backupName, title, { preserveSys = false } = {}) {
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
  $("#threadMeta").classList.add("hidden");

  $("#threadStatusBtn").classList.add("hidden");
  $("#threadLockBtn").classList.add("hidden");
  $("#threadConvertBtn").classList.add("hidden");
  $("#threadExportBtn").classList.add("hidden");
  $("#threadBackupsBtn").classList.add("hidden");
  $("#threadReloadBtn").classList.add("hidden");
  $("#toggleSysBtn").classList.remove("hidden");
  $("#topThreadSep").classList.remove("hidden");
  hideUnlockerCard();
  syncSysButton();
  refreshStatusBadge();
  renderSidebar();

  $("#moreWrap").innerHTML = "";
  $("#chatFeed").innerHTML = feedSkeleton();
  CODE_STORE.length = 0;

  startLoad();
  try {
    const r = await rpc("backup_entries", provider, {
      backup_dir: backupDir,
      limit: 80,
      include_tools: STATE.showSys,
    });
    if (token !== STATE.loadingThreadToken) return;
    STATE.cursor = r.cursor;
    STATE.loadedMsgs = r.page_msgs || 0;
    STATE.active.title = r.title || title || backupName;
    STATE.active.totalLabel = r.total_label || "";
    STATE.active.lastTurns = r.turns || [];
    STATE.active.hasMore = !!r.has_more;

    titleEl.textContent = STATE.active.title;
    titleEl.title = PRIVACY_ON ? "" : STATE.active.title;
    const metaEl = $("#threadMeta");
    if (r.total_label) {
      metaEl.textContent = localizeTotalLabel(r.total_label);
      metaEl.classList.remove("hidden");
    } else {
      metaEl.classList.add("hidden");
    }

    $("#chatFeed").innerHTML = renderTurnsHTML(r.turns || [], provider) ||
      `<div class="list-empty">${esc(t("no_messages"))}</div>`;
    renderMoreBar(r.has_more);

    const sc = $("#feedScroll");
    requestAnimationFrame(() => { sc.scrollTop = sc.scrollHeight; updateScrollBtn(); });
  } catch (e) {
    if (token !== STATE.loadingThreadToken) return;
    $("#chatFeed").innerHTML = `<div class="note err">${ico("alert")}<span>${esc(e.message)}</span></div>`;
  } finally {
    stopLoad();
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
  if (!STATE.active || !STATE.cursor) return;
  const btn = $("#moreBtn");
  if (btn) {
    btn.disabled = true;
    btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("loading"))}</span>`;
  }
  const sc = $("#feedScroll");
  const prevHeight = sc.scrollHeight;
  const prevTop = sc.scrollTop;

  startLoad();
  try {
    const { provider, id, readonlyBackup, backupDir } = STATE.active;
    const r = readonlyBackup
      ? await rpc("backup_entries", provider, {
          backup_dir: backupDir,
          before: STATE.cursor,
          limit: 80,
          include_tools: STATE.showSys,
        })
      : await rpc("entries", provider, {
          cid: id,
          before: STATE.cursor,
          limit: 80,
          include_tools: STATE.showSys,
        });
    STATE.cursor = r.cursor;
    STATE.loadedMsgs += r.page_msgs || 0;
    STATE.active.hasMore = !!r.has_more;
    STATE.active.lastTurns = [...(r.turns || []), ...(STATE.active.lastTurns || [])];
    $("#chatFeed").insertAdjacentHTML("afterbegin", renderTurnsHTML(r.turns || [], provider));
    renderMoreBar(r.has_more);
    sc.scrollTop = prevTop + (sc.scrollHeight - prevHeight);
  } catch (e) {
    toast(e.message, "err");
    renderMoreBar(true);
  } finally {
    stopLoad();
  }
}

function syncSysButton() {
  $("#toggleSysBtn").classList.toggle("active-toggle", STATE.showSys);
}

function updateScrollBtn() {
  const sc = $("#feedScroll");
  const btn = $("#scrollBottomBtn");
  if (!sc || !btn) return;
  const dist = sc.scrollHeight - sc.scrollTop - sc.clientHeight;
  btn.classList.toggle("hidden", dist < 380);
}
$("#feedScroll").addEventListener("scroll", updateScrollBtn, { passive: true });
$("#scrollBottomBtn").onclick = () => $("#feedScroll").scrollTo({ top: $("#feedScroll").scrollHeight, behavior: "smooth" });

let editCtx = null;
async function openEdit(key) {
  if (!STATE.active || STATE.active.readonlyBackup) return;
  const { provider, id } = STATE.active;
  if (!guardNotRunning(provider)) return;
  editCtx = { provider, cid: id, key, revision: null };
  $("#editTitle").textContent = t("loading");
  $("#editText").value = "";
  $("#editText").readOnly = true;
  $("#editApplyBtn").disabled = true;
  openModal("editModal");

  startLoad();
  try {
    const r = await rpc("edit_preview", provider, { cid: id, key });
    editCtx.revision = r.revision;
    $("#editTitle").textContent = localizeKind(r.kind || t("edit"));
    $("#editText").value = r.text ?? "";
    $("#editText").readOnly = !r.writable;
    $("#editApplyBtn").disabled = !r.writable;
    $("#editText").focus();
  } catch (e) {
    closeModal("editModal");
    toast(e.message, "err");
  } finally {
    stopLoad();
  }
}

async function applyEdit() {
  if (!editCtx) return;
  const text = $("#editText").value;
  if (!text.trim()) { toast(t("toast_empty_text"), "err"); return; }
  const btn = $("#editApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("saving"))}</span>`;
  try {
    const r = await rpc("apply_edit", editCtx.provider, {
      cid: editCtx.cid,
      key: editCtx.key,
      text,
      revision: editCtx.revision,
      confirm: true,
    });
    const job = await waitJob(r.job_id);
    closeModal("editModal");
    showReport(t("saved"), job.result);
  } catch (e) {
    toast(e.message, "err");
  } finally {
    btn.disabled = false;
    btn.innerHTML = `${ico("check")}<span>${esc(t("save"))}</span>`;
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
  if (!STATE.active || STATE.active.provider !== "codex") return;
  if (!guardNotRunning(STATE.active.provider)) return;
  if (STATE.active.blockedInfo && STATE.active.blockedInfo.is_blocked) {
    showUnlockerCard(STATE.active.blockedInfo, true);
  } else {
    openLockModal(STATE.active.provider, STATE.active.id);
  }
};

$("#unlockerDismissBtn").onclick = () => hideUnlockerCard();

$("#unlockerApplyBtn").onclick = async () => {
  if (!STATE.active) return;
  const { provider, id } = STATE.active;
  if (!guardNotRunning(provider)) return;
  const btn = $("#unlockerApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("unlocking"))}</span>`;
  try {
    const r = await rpc("unlock_chat", provider, { cid: id });
    await waitJob(r.job_id);
    hideUnlockerCard();
    toast(t("toast_unlocked"), "ok");
    await openThread(provider, id, { preserveSys: STATE.showSys });
  } catch (e) {
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
  if (!lockCtx) return;
  const message = $("#lockReasonInput").value.trim();
  const code = $("#lockCodeInput").value.trim() || "misalignmentPolicyViolation";
  if (!message) {
    toast(t("toast_enter_reason"), "err");
    $("#lockReasonInput").focus();
    return;
  }
  const btn = $("#lockApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("locking"))}</span>`;
  try {
    const r = await rpc("lock_chat", lockCtx.provider, { cid: lockCtx.cid, message, code });
    await waitJob(r.job_id);
    closeModal("lockModal");
    toast(t("toast_locked"), "ok");
    if (STATE.active && STATE.active.provider === lockCtx.provider && STATE.active.id === lockCtx.cid) {
      await openThread(lockCtx.provider, lockCtx.cid, { preserveSys: STATE.showSys });
    } else {
      await openThread(lockCtx.provider, lockCtx.cid, { preserveSys: false });
    }
  } catch (e) {
    toast(e.message, "err");
  } finally {
    btn.disabled = false;
    btn.innerHTML = `${ico("lock")}<span>${esc(t("lock_apply"))}</span>`;
  }
};

function openWelcome() {
  const modal = $("#welcomeModal");
  if (!modal) return;
  modal.classList.remove("open");
  void modal.offsetWidth;
  modal.classList.add("open");
}

function dismissWelcome() {
  closeModal("welcomeModal");
}

$("#welcomeStartBtn")?.addEventListener("click", dismissWelcome);
$("#openWelcomeBtn")?.addEventListener("click", () => openWelcome());

let cutCtx = null;
async function openCut(key, mode) {
  if (!STATE.active || STATE.active.readonlyBackup) return;
  const { provider, id } = STATE.active;
  if (!guardNotRunning(provider)) return;
  cutCtx = { provider, cid: id, key, mode, revision: null, writable: false };

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
    cutCtx.revision = r.revision;
    cutCtx.writable = !!r.writable;
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
    closeModal("cutModal");
    toast(e.message, "err");
  } finally {
    stopLoad();
  }
}

$("#cutApplyBtn").onclick = () => {
  if (!cutCtx || !cutCtx.writable) return;
  const btn = $("#cutApplyBtn");
  handleArmedClick(btn, t("confirm"), async () => {
    btn.disabled = true;
    btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("deleting"))}</span>`;
    try {
      const r = await rpc("apply_cut", cutCtx.provider, {
        cid: cutCtx.cid,
        key: cutCtx.key,
        mode: cutCtx.mode,
        revision: cutCtx.revision,
        confirm: true,
      });
      const job = await waitJob(r.job_id);
      closeModal("cutModal");
      showReport(t("done"), job.result);
    } catch (e) {
      toast(e.message, "err");
    } finally {
      btn.disabled = false;
      btn.innerHTML = `${ico("trash")}<span class="arm-label">${esc(t("delete"))}</span><span class="arm-bar"></span>`;
    }
  });
};

async function executeDeleteSingle(btn, key) {
  if (!STATE.active || STATE.active.readonlyBackup) return;
  const { provider, id } = STATE.active;
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("deleting"))}</span>`;
  try {
    const r = await rpc("delete_single", provider, { cid: id, key, confirm: true });
    await waitJob(r.job_id);
    toast(t("toast_step_deleted"), "ok");
    await openThread(provider, id, { preserveSys: true });
    refreshAll(false);
  } catch (e) {
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
      html: `<span title="${esc(fullPath)}">${esc(shortName)}</span><button type="button" class="rep-copy-btn" data-copy-path="${esc(fullPath)}" title="${esc(t("rep_copy_path"))}" aria-label="${esc(t("rep_copy_path"))}">${ico("copy")}</button>`,
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
        await navigator.clipboard.writeText(btn.dataset.copyPath || "");
        toast(t("rep_path_copied"), "ok");
      } catch {}
    };
  });
  openModal("repModal");
}

$("#repCloseBtn").onclick = async () => {
  closeModal("repModal");
  if (STATE.active && !STATE.active.readonlyBackup) {
    await openThread(STATE.active.provider, STATE.active.id, { preserveSys: true });
  }
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

function triggerDownload(url, filename) {
  const a = document.createElement("a");
  a.href = url;
  a.download = filename || "chat.k1tty";
  document.body.appendChild(a);
  a.click();
  a.remove();
}

$("#exportApplyBtn").onclick = async () => {
  if (!exportCtx) return;
  const btn = $("#exportApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("exporting"))}</span>`;
  startLoad();
  try {
    const resp = await fetch("/api/export_pack", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        provider: exportCtx.provider,
        cid: exportCtx.cid,
        preset: exportCtx.preset,
        include_tools: $("#exportIncludeTools").checked,
      }),
    });
    const data = await resp.json();
    if (!resp.ok || !data.ok) throw new Error(data.error || t("err_op"));
    const st = data.stats || {};
    const dlUrl = `/api/download_pack/${encodeURIComponent(data.token)}`;
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
    triggerDownload(dlUrl, data.filename);
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

function openImportModal() {
  const defaultDst = (STATE.active && !STATE.active.readonlyBackup ? STATE.active.provider : null)
    || PROV_ORDER.find((p) => STATE.providers[p]?.available)
    || "antigravity";
  importCtx = { file: null, bytes: null, info: null, dst: defaultDst };
  $("#importFileInput").value = "";
  $("#importDropTitle").textContent = t("drop_k1tty_hint");
  $("#importDropMeta").textContent = "";
  $("#importInspectCard").classList.add("hidden");
  $("#importInspectCard").innerHTML = "";
  renderProviderPickGrid($("#importTargetGrid"), defaultDst);
  $("#importTitleInput").value = "";
  $("#importIncludeTools").checked = true;
  const btn = $("#importApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("upload")}<span>${esc(t("import_btn"))}</span>`;
  openModal("importModal");
}

async function handleImportFile(file) {
  if (!file || !importCtx) return;
  importCtx.file = file;
  $("#importDropTitle").textContent = file.name;
  $("#importDropMeta").textContent = `${fmtSize(file.size)} · ${t("inspecting")}`;
  $("#importApplyBtn").disabled = true;
  startLoad();
  try {
    const buf = await file.arrayBuffer();
    importCtx.bytes = buf;
    const resp = await fetch("/api/inspect_pack", {
      method: "POST",
      headers: { "Content-Type": "application/octet-stream" },
      body: buf,
    });
    const data = await resp.json();
    if (!resp.ok || !data.ok) throw new Error(data.error || t("err_op"));
    const info = data.info;
    importCtx.info = info;
    $("#importDropMeta").textContent = `${fmtSize(file.size)} · .k1tty v${info.version}`;
    const srcProv = info.source_provider;
    if (srcProv && PROV[srcProv]) {
      importCtx.dst = srcProv;
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
    importCtx.bytes = null;
    importCtx.info = null;
    $("#importDropMeta").textContent = "";
    $("#importInspectCard").classList.add("hidden");
    toast(e.message, "err");
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
  if (!b || !importCtx) return;
  importCtx.dst = b.dataset.pickProv;
  $$("[data-pick-prov]", $("#importTargetGrid")).forEach((x) => {
    x.classList.toggle("active", x === b);
  });
});

$("#importApplyBtn").onclick = async () => {
  if (!importCtx || !importCtx.bytes) return;
  if (!guardNotRunning(importCtx.dst)) return;
  const btn = $("#importApplyBtn");
  btn.disabled = true;
  btn.innerHTML = `${ico("loader", "spin")}<span>${esc(t("importing"))}</span>`;
  startLoad();
  try {
    const qs = new URLSearchParams({
      target: importCtx.dst,
      title: $("#importTitleInput").value.trim(),
      include_tools: $("#importIncludeTools").checked ? "1" : "0",
    });
    const resp = await fetch("/api/import_pack?" + qs.toString(), {
      method: "POST",
      headers: { "Content-Type": "application/octet-stream" },
      body: importCtx.bytes,
    });
    const data = await resp.json();
    if (!resp.ok || !data.ok) throw new Error(data.error || t("err_op"));
    const res = data.result || {};
    closeModal("importModal");
    toast(t("toast_imported", { prov: PROV[res.provider]?.name || importCtx.dst }), "ok");
    await refreshAll(true);
    if (res.provider && res.cid) {
      await openThread(res.provider, res.cid, { preserveSys: false });
    }
  } catch (e) {
    toast(e.message, "err");
  } finally {
    stopLoad();
    btn.disabled = false;
    btn.innerHTML = `${ico("upload")}<span>${esc(t("import_btn"))}</span>`;
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
  renderSidebar();
});
$("#searchClear").onclick = () => {
  $("#searchInput").value = "";
  STATE.searchQuery = "";
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
  openThread(item.dataset.prov, item.dataset.cid, { preserveSys: false });
});

$("#convList").addEventListener("keydown", (ev) => {
  if (ev.key !== "Enter" && ev.key !== " ") return;
  const item = ev.target.closest(".conv-item");
  if (!item || ev.target.closest("[data-more]")) return;
  ev.preventDefault();
  openThread(item.dataset.prov, item.dataset.cid, { preserveSys: false });
});

$("#provGrid").addEventListener("click", (ev) => {
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
  if (!STATE.active) return;
  STATE.showSys = !STATE.showSys;
  syncSysButton();
  if (STATE.active.readonlyBackup) {
    await openBackupResearch(
      STATE.active.provider,
      STATE.active.backupDir,
      STATE.active.backupName,
      STATE.active.title,
      { preserveSys: true }
    );
  } else {
    await openThread(STATE.active.provider, STATE.active.id, { preserveSys: true });
  }
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
    const code = CODE_STORE[Number(actEl.dataset.codeIdx)] ?? "";
    navigator.clipboard?.writeText(code).then(() => {
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
    if ($("#appRoot").classList.contains("drawer-collapsed")) toggleDrawer(true);
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
      track.classList.remove("has-scroll", "visible");
      return;
    }
    track.classList.add("has-scroll");
    const thumbH = Math.max(28, Math.round((ch / sh) * th));
    const maxScroll = Math.max(1, sh - ch);
    const maxY = Math.max(0, th - thumbH);
    const ratio = Math.min(1, Math.max(0, scroller.scrollTop / maxScroll));
    thumb.style.height = thumbH + "px";
    thumb.style.transform = `translateY(${(ratio * maxY).toFixed(1)}px)`;
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
    scroller.scrollTo({ top: ratio * (scroller.scrollHeight - scroller.clientHeight), behavior: "smooth" });
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
  scheduleUpdate();
  return { update: scheduleUpdate, track };
}

function initCustomScrollbars() {
  attachOverlayScrollbar($("#feedScroll"), $("#chatWrap"), { top: 8, bottom: 8, right: 4 });
  attachOverlayScrollbar($("#overviewPane"), $(".main-body"), { top: 8, bottom: 8, right: 4 });
  attachOverlayScrollbar($("#convList"), $("#convListWrap"), { top: 4, bottom: 8, right: 2 });
}

initCustomScrollbars();
applyI18n();
window.addEventListener("DOMContentLoaded", syncCatBtn, { once: true });

async function boot() {
  $("#convList").innerHTML = convListSkeleton();
  renderOverview();
  await refreshAll(false);
  openFromHash();
}

wsConnect();
