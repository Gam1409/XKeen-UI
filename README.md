<div align="center">
  <img width="128" height="128" src="frontend/src/assets/favicon.png" alt="XKeen UI">

<h1>XKeen UI</h1>

<p>Легковесная панель управления сервисом <b>XKeen</b> для роутеров Keenetic/Netzraze</p>
  
![preview](preview.gif)

</div>
<br>  
  
## ✨ Особенности

- 🚀 Установка одной командой
- 📉 Низкое потребление ресурсов
- ⛔ Никаких зависимостей кроме XKeen
- ⚓️ Порт по умолчанию: 1000 (меняется в `/opt/etc/init.d/S99xkeen-ui`)
- 🎛️ Управление сервисом: `/opt/etc/init.d/S99xkeen-ui start|restart|stop|status`

&nbsp;

## ⚙️ Функционал

- 📊 Мониторинг и управление сервисом
- 📝 Редактирование конфигураций с валидацией и форматированием
- 📜 Просмотр логов с автообновлением и фильтрацией
- 🕒 Выбор часового пояса в логах
- 🔀 Переключение/установка/обновление ядер Xray и Mihomo
- 🔗 Генерация аутбаундов из ссылок (также доступно [отдельно по ссылке](https://zxc-rv.github.io/XKeen-UI/Outbound_Generator/))
- 🩻 Сканирование dat файлов
- ⚔️ Clash API реализация для Mihomo

&nbsp;

## ⚡️ Быстрый старт (установка/обновление/удаление)

### Cтабильная/Latest версия

```SH
curl https://raw.githubusercontent.com/zxc-rv/XKeen-UI/main/setup.sh | sh
```

### Бета/Pre-release версия

```SH
curl https://raw.githubusercontent.com/zxc-rv/XKeen-UI/main/setup.sh | sh -s -- beta
```

<br>

## 🌐 Доступ извне

Панель разработана для работы в локальной сети. В случае необходимости использовать панель за пределами локальной сети рекомендуется использовать VPN протоколы, такие как SSTP или Wireguard.
Также поддерживается работа с KeenDNS, для этого нужно в веб-конфигураторе создать саб-домен с протоколом HTTP и портом панели. Обязательно используйте авторизацию и другие меры безопасности!
> [!CAUTION]
> Открытие доступа к панели из интернета без должных мер безопасности может привести к взлому роутера или утечке данных.
> За данные последствия автор проекта ответственность не несет.
<br>
  
## 🪙 Понравился проект? Поддержи разработку

- [**Cloudtips**](https://pay.cloudtips.ru/p/24b4c4b6)

- Банковская карта: `2204 3203 4161 6409`
  
&nbsp;

## 🙏 Благодарности

- [**Skrill0/XKeen**](https://github.com/Skrill0/XKeen)  
- [**jameszeroX/XKeen**](https://github.com/jameszeroX/XKeen)  
- [**Anonym-tsk/nfqws-keenetic**](https://github.com/Anonym-tsk/nfqws-keenetic)

## Подписки

XKeen UI включает управляемую панель подписок.

- `xray/watcher`: хранит метаданные подписок в `/opt/etc/xkeen/subscriptions.json`, запускает `xkeen-subscription-watcher` только через массив аргументов, валидирует итоговый набор конфигов Xray и записывает `/opt/etc/xray/configs/04_outbounds.sub.<id>.json`.
- `xray/native`: скачивает plain/base64-подписки и Xray JSON на стороне backend, строит Xray outbounds для поддерживаемых share-ссылок и показывает предупреждения по ссылкам или параметрам, которые нельзя безопасно выразить в Xray JSON.
- `mihomo/provider`: редактирует только управляемые XKeen-UI блоки в `/opt/etc/mihomo/config.yaml`, валидирует конфиг через `mihomo -t -f` и при необходимости добавляет связанный `proxy-groups`.
- Расписание записывается в crontab блоками `# XKeen-UI subscription <id> BEGIN/END` и запускает `xkeen-ui subscription-update <id>`.
- Обновления используют lock `/opt/var/run/xkeen-ui-subscriptions.lock`; свежий lock блокирует параллельные обновления подписок на 30 минут.
- URL хранятся только в `subscriptions.json` с правами `0600` и маскируются в API-ответах и логах.
- Резервные копии хранятся в `/opt/etc/xkeen/backups/subscriptions/<id>/`; действие в интерфейсе называется `Восстановить`.

Покрытие native parser намеренно консервативное. Генерация Xray output сейчас обрабатывает ссылки VMess, VLESS, Trojan, Shadowsocks, HTTP, SOCKS и Hysteria2, включая параметры Hysteria2 `obfs`, `SNI`, `insecure`, `ALPN`, `pinSHA256` и `mport`, если Xray может провалидировать получившийся конфиг. Протоколы только для Mihomo, такие как TUIC, Snell, AnyTLS, Mieru, SSH, OpenVPN, MASQUE и Tailscale, показываются в preview как предупреждения, если не используются через `mihomo/provider`.
