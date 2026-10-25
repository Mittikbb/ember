import { createContext, Fragment, useContext, type ReactNode } from "react";
import type { Lang } from "./api";

const en = {
  "tab.states": "Statuses",
  "tab.setup": "Setup",
  "toggle.title": "Show the status in Discord",
  "lang.title": "Interface language",

  "game.closed": "Deadlock is not running",
  "now.closed": "not running",
  "now.launching": "starting",
  "now.hideout": "in the Hideout",
  "now.queue": "finding a match",
  "now.found": "match found",
  "now.loading": "match starting",
  "now.match": "in a match",
  "now.postGame": "after the match",
  "now.sandbox": "playing with bots",
  "now.spectating": "spectating",

  "discord.off": "Status is off",
  "discord.connected": "Discord connected",
  "discord.connecting": "Connecting to Discord…",
  "discord.error": "Discord error",
  "discord.waiting": "Waiting for the game",
  "discord.noClientId": "Enter your Discord app's Application ID",
  "discord.waitingGame": "Waiting for Deadlock to start",
  "discord.notRunning": "Discord is not running",
  "discord.admin": "Discord runs as administrator: run Ember the same way, or Discord without admin rights",
  "discord.noReply": "Discord is not responding",
  "discord.closed": "Discord closed the connection",

  "phase.launching": "Starting",
  "phase.hideout": "Hideout",
  "phase.queue": "Finding a match",
  "phase.found": "Match found",
  "phase.loading": "Match starting",
  "phase.match": "In a match",
  "phase.postGame": "Post-game",
  "phase.sandbox": "With bots",
  "phase.spectating": "Spectating",
  "hint.launching": "The game is loading",
  "hint.hideout": "Main menu, your hero",
  "hint.queue": "You pressed Play",
  "hint.found": "Connecting to the server",
  "hint.loading": "Loading and hero reveal",
  "hint.match": "The fight is on",
  "hint.postGame": "Results from deadlock-api",
  "hint.sandbox": "Practice on a local server",
  "hint.spectating": "Only with memory reading",
  "live.now": "now",

  "states.friends": "What friends see · {phase}",
  "states.emptyDisabled": "No status in this state",
  "states.noteLive": "Live data from the game.",
  "states.noteSample": "Example: what the game hasn't told yet is filled in for show.",
  "states.inDiscord": "In Discord now",
  "states.notSent": "Nothing sent",
  "states.gameClosed": "Game not running — no status",
  "states.saving": "Saving…",
  "states.saved": "Saved",
  "states.eyebrow": "Game states",
  "states.title": "Each one gets",
  "states.titleEm": "its own lines.",

  "editor.on": "Show in this state",
  "editor.off": "Don't show (only Discord's own “Playing …”)",
  "editor.reset": "Reset to default:",
  "editor.line1": "First line",
  "editor.line2": "Second line",
  "editor.line1Placeholder": "In a match as {hero}",
  "editor.hint":
    "is a fallback: if the first part lacks something (say, match results aren't in yet), the next one shows. Variables with a dot are known after the match; in the Hideout they're about the last one. Green border: they're there during the match too, with memory reading on.",
  "editor.picture": "Picture",
  "editor.small": "Small badge",
  "editor.timer": "Timer",
  "var.afterTitle": "Known after the match",
  "var.liveTitle": "After the match; during it with memory reading",
  "var.memoryTitle": "Only with memory reading, in Street Brawl",

  "image.heroIcon": "Hero icon",
  "image.heroCard": "Hero portrait",
  "image.rank": "Rank badge",
  "image.deadlock": "Deadlock icon",
  "image.app": "App icon",
  "image.none": "No badge",
  "timer.state": "Time in this state",
  "timer.match": "Match time",
  "timer.queue": "Time in queue",
  "timer.none": "No timer",

  "var.hero": "Hero",
  "var.rank": "Rank",
  "var.hero_line": "Hero line (English)",
  "var.result": "Won / lost",
  "var.kda": "K/D/A",
  "var.souls": "Souls",
  "var.kills": "Kills",
  "var.deaths": "Deaths",
  "var.assists": "Assists",
  "var.last_hits": "Last hits",
  "var.level": "Level",
  "var.duration": "Duration",
  "var.mode": "Mode",
  "var.round": "Street Brawl round",
  "var.score": "Street Brawl score",
  "var.match_id": "Match ID",
  "var.steam_id": "SteamID",
  "var.account_id": "Account ID",

  "card.playing": "Playing",

  "s1.title": "Discord application",
  "s1.intro": "Discord shows “Playing …” with the name of the app that sent the status. Create your own (it takes a minute):",
  "s1.open": "Open",
  "s1.openAfter": "→ **New Application**.",
  "s1.name": "Name it **Deadlock**: friends will see “Playing Deadlock”.",
  "s1.copy": "Copy the **Application ID** and paste it here. No secret or OAuth needed.",
  "s1.badId": "That doesn't look like an Application ID: it's 17–20 digits.",
  "s1.lumenClash":
    "Lumen already uses this ID. The status would then carry the Lumen app's name, and Lumen's music and the game would override each other. Create a separate app for Ember.",
  "icon.title": "Sharp Deadlock icon",
  "icon.intro":
    "By link, Steam only serves the game's icon at 32×32, so it looks blurry in Discord. Discord shows the sharp one (256×256) if you upload it to your app:",
  "icon.save": "Save the icon",
  "icon.saveAfter": "— **deadlock.png** will appear in Downloads.",
  "icon.upload":
    "In the Developer Portal open the app → **Rich Presence** → **Art Assets** → **Add Image(s)** and pick the file. Keep the name **deadlock**. Click Save Changes.",
  "icon.turnOn": "Turn on the switch below. Discord picks up new images within a few minutes.",
  "icon.switch": "I uploaded the icon named deadlock",
  "icon.note": "You can also set this picture as the app's App Icon — then “App icon” is the Deadlock icon too.",

  "s2.found": "Game found via Steam.",
  "s2.foundCustom": "Game found at the given folder.",
  "s2.notFound": "Couldn't find Deadlock in your Steam libraries. Set the game folder by hand.",
  "s2.folder": "Game folder (optional)",
  "s2.condebug": "Ember reads the game's log, so Steam → Deadlock → Properties → Launch Options must include",
  "s2.copied": "copied",
  "s2.safe":
    "The log is the safe source: it doesn't break after patches and doesn't touch the anti-cheat. Ember reads the game's memory only if you turn that on below.",
  "s2.stale": "The game is running but not writing its log: looks like -condebug is missing. Add it and restart Deadlock.",

  "s3.title": "Buttons under the status",
  "s3.intro": "Up to two buttons with links. Others see them, you don't: that's how Discord works.",
  "s3.steam": "Steam profile",
  "s3.custom": "Custom",
  "s3.label": "Label",
  "s3.remove": "Remove",

  "s4.title": "Other",
  "s4.postGame": "“Post-game” stays in the Hideout for, minutes",
  "s4.lang": "Language: the window and the words Ember fills in ({result}, {mode})",
  "s4.autostart": "Start with Windows (in the tray)",

  "mem.title": "For the confident: reading game memory",
  "mem.intro":
    "The log doesn't know what's happening inside a match. With this setting Ember reads Deadlock's memory and learns in real time: the exact hero, K/D/A, souls, level, mode (ranked, Street Brawl), round and score — and that you're spectating someone else's match.",
  "mem.switch": "Read game memory",
  "mem.waiting": "Waiting for Deadlock",
  "mem.loading": "The game is loading a map",
  "mem.attached": "Connected, numbers are being read",
  "mem.stale": "The game updated and the numbers read wrong, so they aren't used until a new Ember",
  "mem.error": "Couldn't connect",
  "memory.admin": "the game runs as administrator: run Ember the same way",

  "risk.title": "Read this before turning it on",
  "risk.1":
    "**This is unofficial.** Ember will open deadlock.exe and read its memory. Only read: it writes nothing, injects no code and hooks no game functions. The reading is done by the open-source deadlock-rs library.",
  "risk.2":
    "**Small but non-zero risk to your account.** Valve hasn't approved such programs, and nobody can guarantee how the anti-cheat treats them. The call is yours. If you worry about your account, stick to the log.",
  "risk.3":
    "**Numbers may break after patches.** Ember notices when it reads obvious garbage and stops showing them until an update comes out. Game phases still come from the log.",
  "risk.4": "**Friends see the numbers live** if you put them in your lines: for example, that you're 0/8 and losing.",
  "risk.5": "If Deadlock runs as administrator, Ember will have to run as administrator too.",
  "risk.check": "I understand the risks and turn it on at my own responsibility",
  "risk.cancel": "Never mind",
  "risk.accept": "Turn on memory reading",
};

export type Key = keyof typeof en;

const ru: Record<Key, string> = {
  "tab.states": "Статусы",
  "tab.setup": "Настройка",
  "toggle.title": "Показывать статус в Discord",
  "lang.title": "Язык интерфейса",

  "game.closed": "Deadlock не запущен",
  "now.closed": "не запущен",
  "now.launching": "запускается",
  "now.hideout": "в хайдауте",
  "now.queue": "ищет матч",
  "now.found": "матч найден",
  "now.loading": "матч начинается",
  "now.match": "в матче",
  "now.postGame": "после матча",
  "now.sandbox": "играет с ботами",
  "now.spectating": "смотрит матч",

  "discord.off": "Статус выключен",
  "discord.connected": "Discord подключён",
  "discord.connecting": "Подключаюсь к Discord…",
  "discord.error": "Ошибка Discord",
  "discord.waiting": "Ждёт игру",
  "discord.noClientId": "Укажи Application ID своего Discord-приложения",
  "discord.waitingGame": "Ждёт запуска Deadlock",
  "discord.notRunning": "Discord не запущен",
  "discord.admin": "Discord запущен от администратора: запусти Ember так же или Discord без прав администратора",
  "discord.noReply": "Discord не отвечает",
  "discord.closed": "Discord закрыл соединение",

  "phase.launching": "Запуск",
  "phase.hideout": "Хайдаут",
  "phase.queue": "Поиск матча",
  "phase.found": "Матч найден",
  "phase.loading": "Начало матча",
  "phase.match": "В матче",
  "phase.postGame": "После матча",
  "phase.sandbox": "С ботами",
  "phase.spectating": "Смотрит матч",
  "hint.launching": "Игра загружается",
  "hint.hideout": "Главное меню, свой герой",
  "hint.queue": "Нажал «Играть»",
  "hint.found": "Подключение к серверу",
  "hint.loading": "Загрузка и представление героев",
  "hint.match": "Бой идёт",
  "hint.postGame": "Итоги с deadlock-api",
  "hint.sandbox": "Тренировка на локальном сервере",
  "hint.spectating": "Только с чтением памяти",
  "live.now": "сейчас",

  "states.friends": "Что увидят друзья · {phase}",
  "states.emptyDisabled": "В этом состоянии статус не показывается",
  "states.noteLive": "Живые данные из игры.",
  "states.noteSample": "Пример: чего ещё нет в игре, подставлено для наглядности.",
  "states.inDiscord": "Сейчас в Discord",
  "states.notSent": "Ничего не отправлено",
  "states.gameClosed": "Игра не запущена — статуса нет",
  "states.saving": "Сохраняю…",
  "states.saved": "Сохранено",
  "states.eyebrow": "Состояния игры",
  "states.title": "У каждого",
  "states.titleEm": "свои строки.",

  "editor.on": "Показывать в этом состоянии",
  "editor.off": "Не показывать (только «Играет в …» от Discord)",
  "editor.reset": "Сбросить к стандартному:",
  "editor.line1": "Первая строка",
  "editor.line2": "Вторая строка",
  "editor.line1Placeholder": "В матче за {hero}",
  "editor.hint":
    "— запасной вариант: если в первой части чего-то не хватает (например, итогов матча ещё нет), покажется следующая. Переменные с точкой известны после матча, в хайдауте — про последний матч. Зелёная рамка: во время матча они тоже есть, если включено чтение памяти.",
  "editor.picture": "Картинка",
  "editor.small": "Маленький значок",
  "editor.timer": "Таймер",
  "var.afterTitle": "Известно после матча",
  "var.liveTitle": "После матча; во время матча — с чтением памяти",
  "var.memoryTitle": "Только с чтением памяти, в Street Brawl",

  "image.heroIcon": "Иконка героя",
  "image.heroCard": "Портрет героя",
  "image.rank": "Значок ранга",
  "image.deadlock": "Иконка Deadlock",
  "image.app": "Значок приложения",
  "image.none": "Без значка",
  "timer.state": "Время в этом состоянии",
  "timer.match": "Время матча",
  "timer.queue": "Время в очереди",
  "timer.none": "Без таймера",

  "var.hero": "Герой",
  "var.rank": "Ранг",
  "var.hero_line": "Фраза героя (англ.)",
  "var.result": "Победа / поражение",
  "var.kda": "У/С/П",
  "var.souls": "Души",
  "var.kills": "Убийства",
  "var.deaths": "Смерти",
  "var.assists": "Помощь",
  "var.last_hits": "Добивания",
  "var.level": "Уровень",
  "var.duration": "Длительность",
  "var.mode": "Режим",
  "var.round": "Раунд Street Brawl",
  "var.score": "Счёт Street Brawl",
  "var.match_id": "ID матча",
  "var.steam_id": "SteamID",
  "var.account_id": "ID аккаунта",

  "card.playing": "Играет",

  "s1.title": "Приложение Discord",
  "s1.intro": "Discord показывает «Играет в …» с названием приложения, от имени которого отправлен статус. Создай своё (это минута):",
  "s1.open": "Открой",
  "s1.openAfter": "→ **New Application**.",
  "s1.name": "Назови его **Deadlock**: друзья увидят «Играет в Deadlock».",
  "s1.copy": "Скопируй **Application ID** и вставь сюда. Секрет и OAuth не нужны.",
  "s1.badId": "Похоже, это не Application ID: там 17–20 цифр.",
  "s1.lumenClash":
    "Этот ID уже использует Lumen. Тогда статус будет называться как приложение Lumen, а музыка из Lumen и игра станут перебивать друг друга. Создай для Ember отдельное приложение.",
  "icon.title": "Чёткая иконка Deadlock",
  "icon.intro":
    "По ссылке Steam отдаёт иконку игры только 32×32, поэтому в Discord она мыльная. Чёткую (256×256) Discord покажет, если загрузить её в твоё приложение:",
  "icon.save": "Сохранить иконку",
  "icon.saveAfter": "— файл **deadlock.png** появится в Загрузках.",
  "icon.upload":
    "В Developer Portal открой приложение → **Rich Presence** → **Art Assets** → **Add Image(s)**, выбери файл. Имя должно остаться **deadlock**. Нажми Save Changes.",
  "icon.turnOn": "Включи переключатель ниже. Discord подхватывает новые картинки за несколько минут.",
  "icon.switch": "Я загрузил иконку с именем deadlock",
  "icon.note": "Эту же картинку можно поставить и как App Icon приложения — тогда «Значок приложения» тоже будет иконкой Deadlock.",

  "s2.found": "Игра найдена через Steam.",
  "s2.foundCustom": "Игра найдена по указанному пути.",
  "s2.notFound": "Не нашёл Deadlock в библиотеках Steam. Укажи папку игры вручную.",
  "s2.folder": "Папка игры (необязательно)",
  "s2.condebug": "Ember читает лог игры, поэтому в Steam → Deadlock → Свойства → Параметры запуска должно быть",
  "s2.copied": "скопировано",
  "s2.safe":
    "Лог — безопасный источник: он не ломается после патчей и не касается античита. Память игры Ember читает, только если ты сам включишь это внизу.",
  "s2.stale": "Игра запущена, но лог не пишется: похоже, нет -condebug. Добавь его и перезапусти Deadlock.",

  "s3.title": "Кнопки под статусом",
  "s3.intro": "До двух кнопок со ссылками. Их видят другие, но не ты сам: так устроен Discord.",
  "s3.steam": "Профиль Steam",
  "s3.custom": "Свой",
  "s3.label": "Подпись",
  "s3.remove": "Убрать",

  "s4.title": "Остальное",
  "s4.postGame": "«После матча» держится в хайдауте, минут",
  "s4.lang": "Язык: окно и слова, которые подставляет Ember ({result}, {mode})",
  "s4.autostart": "Запускать вместе с Windows (в трее)",

  "mem.title": "Для уверенных: чтение памяти игры",
  "mem.intro":
    "Лог не знает, что происходит внутри матча. С этой настройкой Ember читает память Deadlock и узнаёт в реальном времени: точного героя, У/С/П, души, уровень, режим (рейтинг, Street Brawl), раунд и счёт, а ещё — что ты смотришь чужой матч.",
  "mem.switch": "Читать память игры",
  "mem.waiting": "Ждёт запуска Deadlock",
  "mem.loading": "Игра загружает карту",
  "mem.attached": "Подключено, цифры читаются",
  "mem.stale": "Игра обновилась, и цифры читаются неверно: они не используются до нового Ember",
  "mem.error": "Не удалось подключиться",
  "memory.admin": "игра запущена от администратора: запусти Ember так же",

  "risk.title": "Прочитай перед включением",
  "risk.1":
    "**Это неофициально.** Ember откроет процесс deadlock.exe и будет читать его память. Только читать: ничего не записывает, не внедряет код и не перехватывает функции игры. Чтение делает открытая библиотека deadlock-rs.",
  "risk.2":
    "**Риск для аккаунта небольшой, но не нулевой.** Valve такие программы не одобряла, и как к ним отнесётся античит, гарантировать никто не может. Решение — твоё. Если боишься за аккаунт, оставь только лог.",
  "risk.3":
    "**После патчей цифры могут сломаться.** Ember замечает, когда читает явную ерунду, и тогда перестаёт их показывать, пока не выйдет обновление. Фазы игры всё равно берутся из лога.",
  "risk.4": "**Друзья увидят цифры вживую,** если ты вставишь их в строки: например, что ты 0/8 и сейчас проигрываешь.",
  "risk.5": "Если Deadlock запущен от имени администратора, Ember тоже придётся запускать от администратора.",
  "risk.check": "Я понимаю риски и включаю на свою ответственность",
  "risk.cancel": "Не надо",
  "risk.accept": "Включить чтение памяти",
};

const dicts: Record<Lang, Record<Key, string>> = { en, ru };

export const LangContext = createContext<Lang>("en");

export type T = (key: Key, vars?: Record<string, string>) => string;

export function useT(): T {
  const lang = useContext(LangContext);
  return (key, vars) => {
    let text = dicts[lang][key] ?? en[key];
    for (const [k, v] of Object.entries(vars ?? {})) text = text.replace(`{${k}}`, v);
    return text;
  };
}

/** A message from the backend: a known code is translated, anything else is shown as is. */
export function message(t: T, text: string | null | undefined): string | null {
  if (!text) return null;
  return text in en ? t(text as Key) : text;
}

/** Text with **bold** parts. */
export function rich(text: string): ReactNode {
  return text.split("**").map((part, i) => (i % 2 ? <b key={i}>{part}</b> : <Fragment key={i}>{part}</Fragment>));
}
