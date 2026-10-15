# Оценка безопасности

Оценка безопасности — это **необязательный** класс от **A** до **F** (и баллы
от 0 до 100), который показывается рядом с мониторами, для которых несколько
проверок говорят что-то полезное: хосты Linux и Windows с агентом, Proxmox VE и
Proxmox Backup Server, Synology, OPNsense, сайты и Active Directory. Другие
мониторы ничего не показывают.

!!! warning "Ориентир, а не аудит"
    Оценка **приблизительная и неполная**. Она читает только то, что DumbMonit
    уже собирает с монитора, и сравнивает это с несколькими рекомендациями
    производителя. Класс A не означает, что устройство безопасно, а
    отсутствующая проверка не означает, что настройка в порядке. Используйте её
    как повод присмотреться, а когда это важно, — настоящий инструмент оценки
    (PingCastle, Mozilla HTTP Observatory, Lynis, собственный советник по
    безопасности производителя…).

## Как включить

Оценка **по умолчанию выключена**. Администратор включает её в
**Настройки → Учётная запись и безопасность → Оценка безопасности**. После
включения:

- каждый оцениваемый монитор получает небольшой **значок-щит** со своей
  буквой в списке мониторов и значок **Security** с баллами на своей странице;
- щелчок по значку открывает **отчёт** об мониторе: что не проходит, с
  подтверждением и способом исправления, что проходит, шаги по улучшению оценки
  для этого типа мониторов и ссылку на соответствующий раздел ниже.

Выключение скрывает значки, останавливает периодический расчёт, а ряды
`dumbmonit_security_score` перестают записываться.

!!! note "Обновление с 0.1.0-alpha.8 или более ранней версии"
    Раньше оценка была всегда включена и имела собственную страницу
    **Security**. Установка, которая уже следила за оцениваемым монитором,
    сохраняет оценку включённой после обновления; новая установка начинается с
    выключенной оценкой. Прежний адрес `/security` теперь открывает настройку.

## Как вычисляется оценка

Для расчёта оценки DumbMonit никогда ничего дополнительно не запрашивает у
монитора. Проверки читают последние значения, которые коллекторы уже
записали (за последние 6 часов), поэтому оценка ничего не стоит устройству.

У каждой проверки есть категория (экспозиция, обновления, аутентификация,
шифрование, резервное копирование, конфигурация), **серьёзность**, которая
является её весом (critical 10, high 6, medium 3, low 1), результат
(**пройдена**, **не пройдена** или **неизвестно**, когда данные не собираются),
подтверждение, способ исправления и ссылка на страницу производителя, на
которой она основана.

**Оценка** = 100 × (вес пройденных проверок) ÷ (вес пройденных + не пройденных
проверок). **Неизвестные проверки показываются, но никогда не учитываются**:
оценка не должна падать из-за того, что учётной записи мониторинга не хватает
привилегии, и расти из-за того, что ничего не известно. Монитор, у которого
все проверки неизвестны, *не оценивается*.

| Класс | Баллы | Слово |
|---|---|---|
| A | 90–100 | Сильная |
| B | 75–89 | Хорошая |
| C | 60–74 | Удовлетворительная |
| D | 40–59 | Слабая |
| F | 0–39 | Плохая |

**Не пройденная критическая проверка ограничивает класс значением C**,
независимо от баллов.

## История и оповещения

Пока оценка включена, сервер пересчитывает её каждые 15 минут и сохраняет как
`dumbmonit_security_score{target="<id>"}` (а число не пройденных проверок — как
`dumbmonit_security_checks_failed`). Её используют два встроенных правила:
**Security score dropped** (на 10 баллов и более ниже лучшего значения за
последние 24 часа) и **Security score low** (ниже 40). См.
[Правила оповещений](../alerting/rules.md#security-score).

## Проверки и как их улучшить

### Хосты Linux { #linux-hosts }

Машины с агентом. Проверкам SSH и межсетевого экрана нужен агент того же
релиза, что и сервер; со старым агентом они неизвестны.

| Проверка | Серьёзность | Данные |
|---|---|---|
| Security updates installed | high | ожидающие обновления безопасности (там, где менеджер пакетов их различает) |
| No pending updates | low | ожидающие обновления |
| No reboot pending | medium | требуется перезагрузка |
| SSH root login with a password refused | high | `PermitRootLogin` из `sshd -T` (`prohibit-password` проходит) |
| SSH accepts keys only | medium | `PasswordAuthentication`, а также `KbdInteractiveAuthentication` при `UsePAM` |
| Host firewall filters inbound traffic | high | цепочка input в nftables или iptables, которая отбрасывает или отклоняет |
| SELinux enforcing | medium | режим SELinux (неизвестно там, где SELinux нет) |
| Container images up to date | medium | более новый образ в реестре (только при наличии контейнеров) |
| No container image older than 180 days | low | возраст образа (только при наличии контейнеров) |

Чтобы улучшить:

1. Установите ожидающие обновления безопасности, затем включите автоматические
   обновления безопасности: `unattended-upgrades` в Debian и Ubuntu,
   `dnf-automatic` в Fedora и RHEL.
2. Перезагружайтесь после обновлений ядра или библиотек.
3. Убедитесь, что ваш SSH-ключ работает, затем задайте `PermitRootLogin no`,
   `PasswordAuthentication no` и `KbdInteractiveAuthentication no` в
   `/etc/ssh/sshd_config` и перезагрузите sshd.
4. Включите межсетевой экран хоста, отбрасывающий незапрошенный входящий
   трафик: `ufw enable` в Ubuntu, firewalld в Fedora и RHEL, набор правил
   nftables в остальных случаях.
5. Держите SELinux в режиме enforcing там, где его поставляет дистрибутив;
   регулярно загружайте более новые образы контейнеров.

Источники: [CIS Controls](https://www.cisecurity.org/controls),
[sshd_config(5)](https://man.openbsd.org/sshd_config),
[Red Hat — Using SELinux](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/using_selinux/index).

### Хосты Windows { #windows-hosts }

Машины с агентом Windows того же релиза, что и сервер (читается из реестра;
старый агент не сообщает ни одной из этих проверок).

| Проверка | Серьёзность | Данные |
|---|---|---|
| Windows Firewall on for every profile | high | профили домена, частный и общий (сначала групповая политика) |
| No reboot pending | medium | Windows Update или обслуживание компонентов ждёт перезагрузки |
| Remote Desktop requires Network Level Authentication | high | проходит, если удалённый рабочий стол выключен |
| SMBv1 disabled | high | параметр сервера SMBv1 |

Чтобы улучшить:

1. Установите обновления Windows (Параметры → Windows Update) и перезагрузитесь,
   когда будет предложено.
2. Держите брандмауэр Защитника Windows включённым для профилей домена,
   частного и общего.
3. Если удалённый рабочий стол включён, требуйте проверку подлинности на уровне
   сети (NLA) и заходите через VPN, а не напрямую из интернета.
4. Удалите SMBv1: `Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol`.

Источники: [Windows Firewall](https://learn.microsoft.com/en-us/windows/security/operating-system-security/network-security/windows-firewall/),
[SMBv1](https://learn.microsoft.com/en-us/windows-server/storage/file-server/troubleshoot/detect-enable-and-disable-smbv1-v2-v3).

### Proxmox VE { #proxmox-ve }

| Проверка | Серьёзность | Данные |
|---|---|---|
| Security updates installed | high | ожидающие обновления безопасности по узлам (нужно право `Sys.Modify` у токена, иначе неизвестно) |
| Proxmox VE packages up to date | medium | обновляемые пакеты Proxmox по узлам |
| Running the installed kernel | medium | установлено более новое ядро, чем запущенное |
| No test repository in use | medium | включённые стандартные репозитории |
| Package repositories correctly configured | medium | ошибки и предупреждения, которые Proxmox VE выдаёт по репозиториям |
| Enterprise repository subscription | low | состояние подписки по узлам |
| Web interface certificate valid for 14+ days | medium | срок действия сертификата узла |
| Every guest is in a backup job | high | гости, не выбранные ни одним заданием резервного копирования |
| Backup jobs succeed | high | последний результат каждого задания резервного копирования |

Чтобы улучшить:

1. Используйте репозиторий, на который у вас есть право: enterprise с
   подпиской, иначе `pve-no-subscription`; никогда `pvetest` на важном узле.
2. Установите обновления на панели **Updates** узла, затем перезагрузитесь в
   новое ядро.
3. Включите каждую ВМ и контейнер в задание резервного копирования, лучше на
   Proxmox Backup Server, и исправьте любое задание, которое не проходит.
4. Замените самоподписанный веб-сертификат сертификатом ACME.

Источники: [System Software Updates](https://pve.proxmox.com/wiki/System_Software_Updates),
[Package Repositories](https://pve.proxmox.com/wiki/Package_Repositories),
[Backup and Restore](https://pve.proxmox.com/wiki/Backup_and_Restore).

### Proxmox Backup Server { #proxmox-backup-server }

| Проверка | Серьёзность | Данные |
|---|---|---|
| System updates installed | high | ожидающие обновления (нужно право `Sys.Modify` на `/system`) |
| Running the installed Backup Server version | medium | демоны, всё ещё работающие на версии до обновления |
| Verify job scheduled | high | число заданий проверки |
| Prune job scheduled | medium | число заданий prune |
| Backups copied elsewhere (sync or tape) | medium | задания синхронизации и резервного копирования на ленту (правило 3-2-1) |
| Garbage collection clean, no bad chunks | high | последний результат GC и повреждённые чанки по datastore |
| Certificate valid for 14+ days | medium | срок действия сертификата |

Чтобы улучшить:

1. Установите обновления и перезагрузитесь, если изменилось ядро или
   `proxmox-backup`.
2. Запланируйте задание проверки и задание prune на каждом datastore.
3. Держите копию в другом месте: задание синхронизации на второй Backup Server
   или лента.
4. Используйте сертификат ACME для веб-интерфейса.

Источники: [Maintenance tasks](https://pbs.proxmox.com/docs/maintenance.html),
[Remotes and sync](https://pbs.proxmox.com/docs/managing-remotes.html).

### Synology { #synology }

| Проверка | Серьёзность | Данные |
|---|---|---|
| DSM 7 or later | high | версия DSM |
| DSM up to date | high | доступно обновление DSM (только учётная запись администратора) |
| Built-in admin account disabled | medium | состояние учётной записи `admin` (только учётная запись администратора) |
| Auto Block enabled | medium | параметр Auto Block (только учётная запись администратора) |
| DSM firewall enabled | medium | параметр межсетевого экрана (только учётная запись администратора) |
| Hyper Backup task configured | medium | число задач Hyper Backup |
| Hyper Backup tasks succeed | high | последний результат каждой задачи |

Четыре параметра читаются раз в час и только когда DumbMonit входит с
учётной записью администратора; с обычной учётной записью они остаются
неизвестными и не учитываются. Двухэтапная проверка и перенаправление на HTTPS
не читаются.

Чтобы улучшить:

1. Используйте последний релиз DSM 7 для вашей модели и позвольте DSM
   автоматически устанавливать важные обновления (Панель управления →
   Обновление и восстановление).
2. Отключите встроенную учётную запись `admin`; входите под личной учётной
   записью администратора, защищённой двухэтапной проверкой.
3. Включите Auto Block и межсетевой экран (Панель управления → Безопасность) и
   не пробрасывайте порты DSM в интернет: используйте VPN.
4. Делайте резервные копии с помощью Hyper Backup на другое устройство или в
   облако и исправляйте любую задачу, которая не проходит: RAID и снимки — это
   не резервные копии.

Источники: [Product support status](https://www.synology.com/en-global/products/status),
[How to add extra security to your Synology NAS](https://kb.synology.com/en-global/DSM/tutorial/How_to_add_extra_security_to_your_Synology_NAS).

### OPNsense { #opnsense }

| Проверка | Серьёзность | Данные |
|---|---|---|
| Firmware updates installed | high | ожидающие обновления пакетов (неизвестно, пока OPNsense не проверит обновления) |
| On the current major release series | medium | доступна новая основная серия |
| No reboot pending after an update | medium | требуется перезагрузка |
| Packet filter enabled | **critical** | состояние `pf` |

Чтобы улучшить:

1. Проверьте обновления в System → Firmware → Status, установите их и
   перезагрузитесь, когда будет предложено.
2. Переходите на новый основной релиз, когда он выйдет.
3. Держите пакетный фильтр включённым.
4. Разрешайте веб-интерфейс и SSH только из LAN или через VPN, никогда из WAN.

Источники: [OPNsense updates](https://docs.opnsense.org/manual/updates.html),
[OPNsense firewall](https://docs.opnsense.org/manual/firewall.html).

### Сайты { #websites }

Для мониторов **HTTP(S)** и **TLS**, которые согласуют TLS, и для мониторов
**Website health & SEO**. У монитора с обычным HTTP нет оценки.

| Проверка | Серьёзность | Данные |
|---|---|---|
| Certificate valid for 14+ days | high | дней до истечения (мониторы HTTP(S) и TLS) |
| Certificate chain trusted | high | проверка цепочки (мониторы HTTP(S) и TLS) |
| TLS 1.3 negotiated | low | согласованная версия протокола (мониторы HTTP(S) и TLS) |
| Served over HTTPS | high | конечный адрес после перенаправлений (Website health) |
| Strict-Transport-Security header | medium | заголовок с `max-age` больше 0 |
| X-Content-Type-Options: nosniff | low | значение заголовка |
| Protected against clickjacking | low | `X-Frame-Options` или `frame-ancestors` в CSP |
| Content-Security-Policy header | low | применяемая политика (report-only не считается) |

Заголовки читаются только в ответах HTTPS.

Чтобы улучшить:

1. Отдавайте сайт только по HTTPS и перенаправляйте обычный HTTP на HTTPS.
2. Автоматизируйте продление сертификата (ACME, например Let's Encrypt) и
   отдавайте полную цепочку.
3. Добавьте заголовки безопасности в веб-сервере или обратном прокси:
   `Strict-Transport-Security`, `X-Content-Type-Options: nosniff` и
   `Content-Security-Policy` с `frame-ancestors`.
4. Сверьтесь с [Mozilla HTTP Observatory](https://developer.mozilla.org/en-US/observatory),
   который проверяет гораздо больше, чем эта оценка.

Источники: [Mozilla Server Side TLS](https://wiki.mozilla.org/Security/Server_Side_TLS),
[MDN — HTTP headers](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers).

### Active Directory { #active-directory }

Оценивается по находкам в стиле PingCastle коллектора Active Directory:
учётные записи, уязвимые к Kerberoasting, учётные записи без предварительной
аутентификации Kerberos, неограниченное делегирование, возраст пароля `krbtgt`,
размер привилегированных групп, охват LAPS, политика паролей, устаревшие
учётные записи. Каждая находка — это проверка, которая проходит, если затрагивает
ноль объектов.

Чтобы улучшить:

1. Сначала исправьте критические и высокие находки: это обычные пути к Domain
   Admins.
2. Если пароль `krbtgt` старый, сбросьте его дважды с интервалом в несколько
   часов и уберите неограниченное делегирование с компьютеров, не являющихся
   контроллерами домена.
3. Переведите службы, работающие под учётными записями пользователей с SPN, на
   групповые управляемые учётные записи служб (gMSA).
4. Запустите [PingCastle](https://www.pingcastle.com/) для полной оценки.

Источники: [PingCastle health check rules](https://www.pingcastle.com/PingCastleFiles/ad_hc_rules_list.html),
[Windows LAPS](https://learn.microsoft.com/en-us/windows-server/identity/laps/laps-overview).

## Ограничения

- Оценка отражает только то, что видит DumbMonit; многие рекомендации не
  проверяются, потому что ни один коллектор не читает нужные данные:
  межсетевой экран и двухфакторная аутентификация Proxmox, веб-интерфейс
  OPNsense, доступный из WAN, контейнеры Docker, запущенные с привилегиями или
  со смонтированным сокетом Docker, всё ещё предлагаемые TLS 1.0/1.1 (проба
  согласует только TLS 1.2 и 1.3) и многое другое.
- Проверка свежа настолько, насколько свежи её данные: некоторые коллекторы
  перечитывают обновления лишь раз в несколько часов.
- Веса — продуктовое решение, одинаковое для всех установок.
- Оцениваются только перечисленные выше типы. Остальные (коммутаторы,
  принтеры, UniFi, pfSense…) намеренно не показывают значка: пара проверок
  дала бы класс, который ничего не значит.
