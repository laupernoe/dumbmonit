# Правила

Правила добавляются при первом запуске, после чего принадлежат вам: DumbMonit
никогда не переписывает изменённое вами правило, он лишь воссоздаёт
отсутствующие. Встроенные правила можно редактировать и отключать, но нельзя
удалять.

## Встроенные правила

Важность показана словом из интерфейса; значение API указано в скобках. Правило,
метрику которого не производит ни одно устройство, не имеет рядов и никогда не
срабатывает, поэтому каждое правило ниже поставляется включённым на каждом
экземпляре.

### Каждое устройство

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Device unreachable | Ни одного измерения не получено более трёх минут. `bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d]))` | > 180 с | 1 мин | Warning (`critical`) | 6 ч |
| High CPU | Нагрузка CPU стабильно выше 90%, усреднённая по устройству по источникам SNMP, Proxmox и агента; сбрасывается ниже 85 %. | > 90 % | 10 мин | Advisory (`warning`), эскалируется через 1 ч | 6 ч |
| Unusual CPU | Нагрузка CPU заметно отличается от обычной для этого времени и дня недели (сезонная базовая линия). | score > 3.5 | 15 мин | Info (`info`) | 6 ч |
| Disk almost full | Файловая система заполнена на 90% или более, по точке монтирования (SNMP, хранилища Proxmox и корневые файловые системы); сбрасывается ниже 88 %. | ≥ 90 % | 15 мин | Advisory (`warning`), эскалируется через 24 ч | 6 ч |
| Filesystem almost full | При такой скорости файловая система заполнится в течение четырёх дней (`predict_linear` за 6 ч; только растущие файловые системы). | ≥ 100 % | 30 мин | Advisory (`warning`) | 6 ч |

### ИБП {#ups}

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| UPS on battery | ИБП питает нагрузку от батареи (`dumbmonit_ups_output_source == 5`). | > 0 | 30 с | Warning (`critical`) | 15 мин |
| UPS battery low | Батарея ИБП разряжена или исчерпана (`dumbmonit_ups_battery_status`, 3 = низкий заряд, 4 = исчерпана). | ≥ 3 | 1 мин | Warning (`critical`) | 30 мин |

### Сетевые порты (SNMP)

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Port accumulating errors | Сетевой порт зафиксировал более пятидесяти ошибок за последний час: обычно это неисправный кабель, грязное оптоволокно или несовпадение duplex (`increase_prometheus(dumbmonit_if_errors_in[1h]) + increase_prometheus(dumbmonit_if_errors_out[1h])`; в течение первого часа нового устройства ничего не срабатывает, поэтому ошибки, подсчитанные до его добавления, не сообщаются как новые). | > 50 | 15 мин | Advisory (`warning`) | 24 ч |
| Port flapping | Сетевой порт упал и снова поднялся более двух раз за тридцать минут (`changes_prometheus(dumbmonit_if_oper_status[30m])`). Порт, отключённый намеренно, не имеет ряда и никогда не срабатывает. | > 4 | 5 мин | Advisory (`warning`) | 1 ч |

### Аппаратное обеспечение сервера (Redfish) {#server-hardware-redfish}

Ряды состояния равны 0 (OK), 1 (Warning) или 2 (Critical). Пустой слот (`Absent`) или отключённый не имеет ряда, поэтому сервер, поставленный с одним блоком питания из двух, никогда не срабатывает.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Server fan failed | Контроллер управления сообщает о неисправности вентилятора (`dumbmonit_redfish_fan_health`). | ≥ 2 | 2 мин | Warning (`critical`) | 6 ч |
| Server temperature above critical | Датчик температуры выше критического порога, который объявляет сам контроллер (`dumbmonit_redfish_temperature_celsius >= dumbmonit_redfish_temperature_upper_critical_celsius`). | > 0 | 5 мин | Warning (`critical`) | 6 ч |
| Power supply redundancy lost | Блоки питания больше не резервируют друг друга: ещё один отказ, и сервер выключится (`dumbmonit_redfish_power_redundancy_health`). | ≥ 1 | 5 мин | Advisory (`warning`), эскалируется через 1 ч | 6 ч |
| Power supply failed | Контроллер сообщает о неисправности блока питания (`dumbmonit_redfish_psu_health`). | ≥ 2 | 2 мин | Warning (`critical`) | 6 ч |
| Drive failure predicted | Диск сообщает о прогнозируемом отказе: замените его, пока он ещё работает (`dumbmonit_redfish_drive_failure_predicted`). | > 0 | 10 мин | Advisory (`warning`) | 24 ч |
| Server health critical | Контроллер сообщает о состоянии самой системы как Critical (`dumbmonit_redfish_system_health`). | ≥ 2 | 2 мин | Warning (`critical`) | 6 ч |

### Мониторы доступности и heartbeat

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Service down | Служба не отвечала корректно три минуты (`dumbmonit_probe_success{probe!="push"} == bool 0`). | > 0 | 3 мин | Warning (`critical`) | 30 мин |
| Heartbeat missed | Задание не вышло на связь в течение ожидаемого интервала плюс льготного периода или само сообщило о сбое (`dumbmonit_probe_success{probe="push"} == bool 0`; задержка задаётся для каждого устройства, см. [Heartbeat](../devices/push.md)). | > 0 | 2 мин | Advisory (`warning`) | 6 ч |
| Service flapping | Служба сменила состояние более шести раз за тридцать минут (`changes(dumbmonit_probe_success{probe!="push"}[30m])`). | > 6 | 5 мин | Advisory (`warning`) | 1 ч |
| Slow service | Служба отвечает дольше трёх секунд; неудачные проверки не учитываются (`dumbmonit_probe_duration_seconds and dumbmonit_probe_success == 1`). | > 3 с | 10 мин | Advisory (`warning`) | 6 ч |
| Certificate expiring soon | Сертификат истекает менее чем через четырнадцать дней. | < 14 дн | 1 ч | Advisory (`warning`) | 24 ч |
| Certificate expired | Срок действия сертификата истёк. | < 0 дн | 5 мин | Warning (`critical`) | 24 ч |

### Контейнеры Docker

Сообщаются агентом; см. [Агент](../devices/agent.md). Оповещение называет
контейнер.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Container stopped | Контейнер остановлен в течение двух минут (`dumbmonit_container_up == bool 0`). Контейнер, который Docker не стал бы перезапускать, не учитывается (`dumbmonit_container_stop_expected = 1`): одноразовый `docker run`, завершившийся штатно, контейнер, который так и не запускали, или остановленный вручную без политики перезапуска. Сбой (любой другой код выхода), завершение по нехватке памяти или контейнер с `always` / `unless-stopped` по-прежнему учитываются. | > 0 | 2 мин | Advisory (`warning`) | 1 ч |
| Container unhealthy | Проверка работоспособности контейнера не проходит три минуты (`dumbmonit_container_health == 2`; 0 нет, 1 здоров, 3 запускается). Остановленный контейнер не учитывается: Docker хранит его последний вердикт, который уже ничего не значит. | > 0 | 3 мин | Advisory (`warning`) | 1 ч |
| Container restarting | Контейнер перезапускался три раза или более за пятнадцать минут (`increase(dumbmonit_container_restart_count[15m])`): он редко остаётся остановленным достаточно долго для предыдущего правила. | ≥ 3 | 1 мин | Advisory (`warning`) | 1 ч |
| Container update available | В реестре доступен более новый образ для этого контейнера (`dumbmonit_container_update_available == 1`). | > 0 | 1 ч | Info (`info`) | 24 ч |

### Резервные копии Plakar

Сообщаются агентом, по kloset и по источнику. Ни одного из этих рядов не
существует, пока Plakar не обнаружен, поэтому ни одно из правил не может
сработать на машине без него.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Plakar backup too old | Нет нового снимка Plakar для этого источника более двух дней (`dumbmonit_backup_last_success_seconds`). | > 2 дн | 1 ч | Advisory (`warning`) | 24 ч |
| Plakar backup failed | Последний снимок содержит ошибки (`dumbmonit_backup_last_status{source!="*"} < 1`). | < 1 | 10 мин | Warning (`critical`) | 24 ч |
| Plakar kloset unreadable | Агент не может открыть kloset: хранилище недоступно, не хватает парольной фразы или плагина; в журнале агента указано, чего именно, один раз (`dumbmonit_backup_last_status{source="*"} < 1`). | < 1 | 30 мин | Advisory (`warning`) | 24 ч |

### Резервные копии restic и Borg

Сообщаются агентом для каждого репозитория, объявленного в `restic_repos` или
`borg_repos` в agent.yaml; см. [Агент](../devices/agent.md#restic-and-borg-backups).
Без объявленного репозитория рядов нет, поэтому ни одно из правил не может
сработать.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| restic/Borg backup too old | Нет нового снимка restic или Borg в этом репозитории более 48 часов (`dumbmonit_agent_backup_repo_last_snapshot_age_seconds`). | > 48 ч | 1 ч | Advisory (`warning`) | 24 ч |
| restic/Borg repository unreachable | Агент не может прочитать репозиторий: он недоступен, пароль отклонён или отсутствует бинарный файл; в логе агента указано, что именно (`dumbmonit_agent_backup_repo_reachable < 1`). | < 1 | 30 мин | Advisory (`warning`) | 24 ч |

### Туннели WireGuard

Сообщаются агентом; см. [Агент](../devices/agent.md#wireguard-tunnels).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| WireGuard peer silent | Peer с persistent keepalive не завершал handshake более 15 минут: туннель не работает (`dumbmonit_agent_wireguard_peer_handshake_age_seconds and on (target, interface, peer) dumbmonit_agent_wireguard_peer_keepalive_seconds > 0`). Peer, который никогда не подключался, отсчитывает свою тишину с момента, когда агент впервые его увидел. Peer без keepalive (телефон, ноутбук) простаивают днями и исключены; измените порог, чтобы изменить N, или уберите часть `and on …`, чтобы следить за каждым peer. | > 15 мин | 5 мин | Advisory (`warning`) | 6 ч |

### Туннели IPsec {#ipsec-tunnels}

Сообщаются агентом; см. [Агент](../devices/agent.md#ipsec-tunnels). Ряд
существует только для CHILD_SA, объявленного в конфигурации strongSwan, поэтому
правило срабатывает и на туннеле, который так и не поднялся, а не только на оборвавшемся.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| IPsec tunnel down | У настроенного туннеля IPsec нет установленной сессии (`dumbmonit_agent_ipsec_tunnel_up < 1`). | < 1 | 10 мин | Advisory (`warning`) | 6 ч |

### Тест скорости Интернета {#internet-speed-test}

Сообщается агентом; см. [Агент](../devices/agent.md#internet-speed-test).
Тест по умолчанию выключен, и каждое правило молчит, пока в `agent.yaml` не задан
соответствующий порог: никто не может угадать, что значит «медленно» для вашего канала.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Internet download rate low | Последний тест показал скорость загрузки ниже `speedtest_min_download_mbps` (`dumbmonit_agent_speedtest_download_min_bytes_per_second - dumbmonit_agent_speedtest_download_bytes_per_second > 0`). | > 0 | 10 мин | Advisory (`warning`) | 24 ч |
| Internet upload rate low | То же, для `speedtest_min_upload_mbps`. | > 0 | 10 мин | Advisory (`warning`) | 24 ч |
| Internet latency high | Последний тест показал задержку выше `speedtest_max_latency_ms`. | > 0 | 10 мин | Advisory (`warning`) | 24 ч |

### Диски агента, ZFS и датчики

Сообщаются агентом; см. [Агент](../devices/agent.md).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Disk SMART failing | Собственная самооценка SMART диска сообщает о его неисправности (`dumbmonit_agent_disk_smart_ok == bool 0`). Замените его и проверьте резервные копии. | > 0 | 10 мин | Warning (`critical`) | 24 ч |
| ZFS pool degraded | Пул ZFS больше не в порядке: устройство отсутствует, неисправно или удалено (`dumbmonit_agent_zfs_pool_health`: 0 online, 1 degraded, 2 всё остальное). | > 0 | 5 мин | Warning (`critical`) | 6 ч |
| ZFS scrub found errors | Последний scrub ZFS нашёл ошибки в этом пуле или файлы безвозвратно потеряны (`dumbmonit_agent_zfs_pool_scrub_errors or dumbmonit_agent_zfs_pool_data_errors`). Пул может оставаться `ONLINE`. | > 0 | 10 мин | Warning (`critical`) | 24 ч |
| Temperature above critical | Датчик выше критической температуры, которую объявляет его собственное оборудование (`dumbmonit_agent_sensor_temperature_celsius >= dumbmonit_agent_sensor_temperature_critical_celsius`); фиксированного значения нет. | > 0 | 5 мин | Warning (`critical`) | 6 ч |

### Proxmox VE

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Backup too old | Нет успешной резервной копии Proxmox VE более семи дней (`dumbmonit_proxmox_backup_last_age_seconds`). | > 7 дн | 1 ч | Advisory (`warning`) | 24 ч |
| VM or container stopped | Гость работал в последние два часа и остановлен пять минут (`1 - dumbmonit_proxmox_guest_running`, объединено через and с `max_over_time(…[2h]) == 1`). | > 0 | 5 мин | Advisory (`warning`) | 6 ч |
| VM or container CPU high | Гость использовал более 90 % выделенных ему ядер в течение пятнадцати минут (`dumbmonit_proxmox_guest_cpu_percent`). Уведомление называет гостя, `nextcloud (202)`. | > 90 % | 15 мин | Advisory (`warning`) | 6 ч |
| VM or container memory high | Гость использовал более 95 % своей памяти в течение десяти минут (`dumbmonit_proxmox_guest_memory_percent`). | > 95 % | 10 мин | Advisory (`warning`) | 6 ч |
| VM or container disk almost full | Корневой диск гостя заполнен более чем на 90 % (`dumbmonit_proxmox_guest_disk_used_percent`): у контейнеров всегда, у ВМ только когда гостевой агент QEMU сообщает об использовании. | > 90 % | 15 мин | Advisory (`warning`) | 24 ч |
| Guest locked | ВМ или контейнер заблокирован шесть часов (`dumbmonit_proxmox_guest_locked`, метка `lock`): прерванная резервная копия или миграция оставила блокировку, и ни одна операция не может выполняться на госте, даже следующая резервная копия. | > 0 | 6 ч | Advisory (`warning`) | 24 ч |
| Old snapshot | Самому старому снимку гостя более тридцати дней (`dumbmonit_proxmox_guest_snapshot_oldest_age_seconds`). | > 30 дн | 1 ч | Info (`info`) | 7 дн |
| HA resource in error | Ресурс Proxmox HA находится в состоянии error или fenced (`dumbmonit_proxmox_ha_resource_error`). | > 0 | 2 мин | Warning (`critical`) | 1 ч |
| HA manager not reporting | Локальный менеджер ресурсов HA узла перестал записывать своё состояние (`dumbmonit_proxmox_ha_lrm_stale`): гости HA этого узла не будут ни перезапущены, ни перемещены. | > 0 | 5 мин | Warning (`critical`) | 1 ч |
| Cluster lost quorum | Кластер Proxmox не имеет кворума (`1 - dumbmonit_proxmox_cluster_quorate`): гостей больше нельзя запускать. | > 0 | 1 мин | Warning (`critical`) | 30 мин |
| Proxmox node offline | Узел кластера сообщается как офлайн (`1 - dumbmonit_proxmox_node_up`). | > 0 | 2 мин | Warning (`critical`) | 30 мин |
| Proxmox service down | Основной демон Proxmox не запущен на узле (`dumbmonit_proxmox_node_core_services_down`: `pve-cluster`, `pvedaemon`, `pveproxy`, `pvestatd`, `pve-firewall`, `corosync`, `watchdog-mux`, каждый учитывается только там, где узел его включает). | > 0 | 5 мин | Warning (`critical`) | 1 ч |
| Node network interface down | Интерфейс, настроенный на запуск при загрузке, не поднят на узле (`dumbmonit_proxmox_node_interface_offline`). Интерфейс, поднимаемый по требованию, никогда не сообщается. | > 0 | 5 мин | Advisory (`warning`) | 6 ч |
| Proxmox storage almost full | Хранилище Proxmox заполнено более чем на 85% (`dumbmonit_proxmox_storage_used_percent`); при 90% подключается "Disk almost full". | > 85 % | 15 мин | Advisory (`warning`) | 24 ч |
| Thin pool almost full | Thin-пул LVM заполнен более чем на 90 %: при заполнении каждый гость, хранящийся на нём, переходит в режим только для чтения (`dumbmonit_proxmox_node_thinpool_used_percent`); сбрасывается ниже 88 %. | > 90 % | 15 мин | Warning (`critical`) | 6 ч |
| Thin pool metadata almost full | Том метаданных thin-пула LVM заполнен более чем на 80 % (`dumbmonit_proxmox_node_thinpool_metadata_used_percent`). Он намного меньше тома данных и часто переполняется первым, с теми же последствиями. | > 80 % | 15 мин | Warning (`critical`) | 6 ч |
| Proxmox disk wearing out | SSD или NVMe узла израсходовал более 90 % своего номинального ресурса (`dumbmonit_proxmox_node_disk_wearout_percent`). Уведомление выглядит как `pve1 · /dev/nvme0n1`. | > 90 % | 1 ч | Advisory (`warning`) | 7 дн |
| Proxmox disk SMART failure | Диск узла сообщает состояние SMART `FAILED` (`dumbmonit_proxmox_node_disk_smart_failed`). | > 0 | 5 мин | Warning (`critical`) | 24 ч |
| Proxmox ZFS pool degraded | Пул ZFS узла не в состоянии `ONLINE` (`dumbmonit_proxmox_node_zfs_pool_degraded`). Уведомление выглядит как `pve2 · tank`. | > 0 | 5 мин | Warning (`critical`) | 6 ч |
| Backup job failed | Последнее задание vzdump на узле или плановое задание резервного копирования завершилось неудачей (`1 - dumbmonit_proxmox_backup_job_last_ok`). | > 0 | 10 мин | Warning (`critical`) | 24 ч |
| Backup job excludes a disk | Плановое задание резервного копирования пропускает диск одного из охватываемых им гостей (`dumbmonit_proxmox_backup_job_guest_excluded_volumes`): задание каждую ночь завершается успешно, а этих данных в копии нет. Приводы CD-ROM и образы cloudinit исключены по замыслу и никогда не учитываются. | > 0 | 1 ч | Advisory (`warning`) | 7 дн |
| Replication failed | Задание репликации сообщает об ошибке или ненулевом счётчике сбоев (`dumbmonit_proxmox_replication_job_error`): резервная копия устарела. | > 0 | 10 мин | Warning (`critical`) | 6 ч |
| Ceph health error | Ceph сообщает `HEALTH_ERR` (`dumbmonit_proxmox_ceph_health`, 0 OK, 1 WARN, 2 ERR). | ≥ 2 | 2 мин | Warning (`critical`) | 30 мин |
| Ceph health warning | Ceph сообщает `HEALTH_WARN` более пятнадцати минут (`dumbmonit_proxmox_ceph_health == 1`). | > 0 | 15 мин | Advisory (`warning`) | 6 ч |
| Ceph OSD down | OSD Ceph не работает пять минут (`1 - dumbmonit_proxmox_ceph_osd_up`). Уведомление называет OSD и его хост. | > 0 | 5 мин | Warning (`critical`) | 1 ч |
| Ceph OSD nearly full | Один OSD Ceph заполнен более чем на 85 % (`dumbmonit_proxmox_ceph_osd_used_percent`): запись прекращается, когда он достигает своего предела, что бы ни говорило среднее по кластеру; сбрасывается ниже 82 %. | > 85 % | 15 мин | Advisory (`warning`) | 24 ч |
| Ceph pool almost full | Пул Ceph заполнен более чем на 85 % (`dumbmonit_proxmox_ceph_pool_used_percent`); сбрасывается ниже 82 %. | > 85 % | 15 мин | Advisory (`warning`) | 24 ч |
| Ceph noout flag left on | Флаг Ceph `noout` установлен два часа (`dumbmonit_proxmox_ceph_flag{flag="noout"}`): ребалансировка отключена, и кластер выглядит здоровым, пока его избыточность сокращается. | > 0 | 2 ч | Info (`info`) | 24 ч |
| Proxmox updates pending | На узле ожидает более двадцати обновлений пакетов (`dumbmonit_proxmox_node_updates_pending`). | > 20 | 1 ч | Info (`info`) | 7 дн |
| Proxmox security updates pending | Хотя бы одно ожидающее обновление приходит из архива безопасности (`dumbmonit_proxmox_node_updates_security_pending`). | > 0 | 1 ч | Advisory (`warning`) | 7 дн |
| Proxmox packages changed | Установленная версия пакета Proxmox изменилась между двумя опросами — кто-то выполнил `apt upgrade` (`dumbmonit_proxmox_node_packages_changed`, публикуется в течение одного часа, подробности в его метке `changes`: `pve1 · pve-manager 8.2.4→8.2.7`). Разрешается само через час. | > 0 | 1 мин | Info (`info`) | 24 ч |
| Node certificate expiring | Сертификат узла Proxmox истекает менее чем через четырнадцать дней (`dumbmonit_proxmox_node_certificate_expiry_days`). | < 14 дн | 1 ч | Advisory (`warning`) | 24 ч |

### Proxmox Backup Server

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| PBS datastore almost full | Datastore PBS заполнен более чем на 90%. | > 90 % | 15 мин | Advisory (`warning`), эскалируется через 24 ч | 6 ч |
| PBS datastore filling up | При текущей скорости PBS оценивает, что datastore заполнится в течение семи дней. | < 7 дн | 1 ч | Advisory (`warning`) | 24 ч |
| PBS datastore not mounted | Съёмный datastore не смонтирован (`dumbmonit_pbs_datastore_removable_unmounted`): в него ничего нельзя записать. | > 0 | 6 ч | Advisory (`warning`) | 24 ч |
| PBS backup too old | Нет нового снимка для этой группы резервного копирования (по одной на машину) более двух дней (`dumbmonit_pbs_backup_last_age_seconds`). Измените порог для машин, копируемых еженедельно; уведомление называет группу и datastore. | > 2 дн | 1 ч | Advisory (`warning`) | 24 ч |
| PBS backup verification failed | Проверка последнего снимка для этой машины завершилась неудачей (`dumbmonit_pbs_backup_last_verified < 1`). | < 1 | 30 мин | Warning (`critical`) | 24 ч |
| PBS task failed | Хотя бы одна задача PBS завершилась неудачей в окне просмотра (`dumbmonit_pbs_tasks_failed`). | > 0 | 10 мин | Advisory (`warning`) | 24 ч |
| PBS garbage collection too old | Нет успешной сборки мусора на этом datastore более восьми дней (`dumbmonit_pbs_gc_last_success_age_seconds`). | > 8 дн | 1 ч | Advisory (`warning`) | 24 ч |
| PBS garbage collection failed | Последняя сборка мусора на datastore завершилась неудачей (`dumbmonit_pbs_gc_last_run_ok < 1`; из `/gc` на PBS 3.3+, из последней задачи GC раньше). | < 1 | 10 мин | Advisory (`warning`) | 24 ч |
| PBS corrupt chunks found | Сборка мусора нашла нечитаемые чанки на datastore (`dumbmonit_pbs_gc_bad_chunks`): некоторые резервные копии больше нельзя восстановить. | > 0 | 10 мин | Warning (`critical`) | 24 ч |
| PBS prune job failed | Последний запуск задания prune PBS завершился неудачей: ничего не очищается, и datastore продолжает заполняться (`dumbmonit_pbs_job_last_ok{kind="prune"} < 1`). Уведомление называет задание и datastore. | < 1 | 10 мин | Advisory (`warning`) | 24 ч |
| PBS verification job failed | Последний запуск задания проверки PBS завершился неудачей (`dumbmonit_pbs_job_last_ok{kind="verify"} < 1`). | < 1 | 10 мин | Advisory (`warning`) | 24 ч |
| PBS sync job failed | Последний запуск задания синхронизации PBS завершился неудачей (`dumbmonit_pbs_sync_job_last_ok < 1`). | < 1 | 10 мин | Warning (`critical`) | 24 ч |
| PBS tape backup failed | Последний запуск резервного копирования на ленту завершился неудачей (`dumbmonit_pbs_tape_backup_job_last_ok < 1`): офлайн-копия не создаётся. | < 1 | 10 мин | Warning (`critical`) | 24 ч |
| PBS job never ran | Задание включено и запланировано, но ни разу не запускалось (`dumbmonit_pbs_job_never_run`). | > 0 | 2 дн | Advisory (`warning`) | 7 дн |
| PBS service down | Служба, необходимая Proxmox Backup Server, не запущена (`dumbmonit_pbs_node_service_active{expected="1"} < 1`); пока она не работает, резервное копирование невозможно. | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| PBS disk SMART failure | Диск сервера резервного копирования сообщает состояние SMART `FAILED` (`dumbmonit_pbs_node_disk_smart_failed`). Уведомление выглядит как `/dev/sdb`. | > 0 | 5 мин | Warning (`critical`) | 24 ч |
| PBS SSD worn out | SSD сервера резервного копирования израсходовал более 90 % своего номинального ресурса (`dumbmonit_pbs_node_disk_wearout_percent`). | > 90 % | 1 ч | Advisory (`warning`) | 7 дн |
| PBS ZFS pool degraded | Пул ZFS сервера резервного копирования не в состоянии `ONLINE` (`dumbmonit_pbs_node_zfs_pool_degraded`). | > 0 | 5 мин | Warning (`critical`) | 6 ч |
| PBS certificate expiring | Сертификат, который отдаёт сервер резервного копирования, истекает менее чем через три недели (`dumbmonit_pbs_node_certificate_expires_seconds`). Требуется параметр `certificates`. | < 21 дн | 1 ч | Advisory (`warning`) | 48 ч |
| PBS restart pending after upgrade | Пакет обновлён, но работающий демон всё ещё старый (`dumbmonit_pbs_node_running_version_stale`). | > 0 | 6 ч | Info (`info`) | 7 дн |
| PBS updates pending | На сервере резервного копирования ожидает более двадцати обновлений пакетов (`dumbmonit_pbs_node_updates_pending`). | > 20 | 1 ч | Info (`info`) | 7 дн |

### Proxmox Datacenter Manager

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Federated instance unreachable | Консоль дата-центра не может достучаться до одного из кластеров Proxmox VE или серверов резервного копирования, которые она объединяет (`dumbmonit_pdm_remote_reachable < 1`). Уведомление называет экземпляр и его продукт. | < 1 | 10 мин | Warning (`critical`) | 6 ч |
| Federated instance behind | Экземпляр работает на более старой версии, чем другой экземпляр того же продукта в той же инфраструктуре (`dumbmonit_pdm_remote_version_behind`). Сравнивается между экземплярами, никогда с тем, что публикует Proxmox. | > 0 | 1 ч | Info (`info`) | 7 дн |
| Task failed on a federated instance | Хотя бы одна задача завершилась неудачей на объединённом экземпляре в окне просмотра (`dumbmonit_pdm_remote_tasks_failed`). | > 0 | 10 мин | Advisory (`warning`) | 24 ч |
| Datacenter console disk almost full | Корневая файловая система консоли дата-центра заполнена более чем на 90 % (`dumbmonit_pdm_node_rootfs_percent`): когда она заполнена, консоль перестаёт записывать то, что собирает. | > 90 % | 15 мин | Advisory (`warning`), эскалируется через 24 ч | 6 ч |
| Datacenter console certificate expiring | Сертификат консоли дата-центра истекает менее чем через четырнадцать дней (`dumbmonit_pdm_node_certificate_expiry_days`). | < 14 дн | 1 ч | Advisory (`warning`) | 24 ч |
| Datacenter console updates pending | На консоли дата-центра ожидает более двадцати обновлений пакетов (`dumbmonit_pdm_node_updates_pending`). | > 20 | 1 ч | Info (`info`) | 7 дн |

### Proxmox Mail Gateway

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Mail queue growing | Очередь отложенной почты выросла более чем на сто сообщений за два часа (`delta(dumbmonit_pmg_queue_messages{queue="deferred"}[2h])`): следующий узел, вероятно, отказывается принимать почту. Рост, а не размер, потому что загруженный шлюз всегда держит несколько десятков отложенных. | > 100 | 15 мин | Advisory (`warning`) | 6 ч |
| Mail stuck in the queue | Сообщение ждёт в очереди более четырёх часов (`dumbmonit_pmg_queue_oldest_age_seconds`). Возраст — нижняя граница (qshape сообщает интервалами), поэтому правило срабатывает скорее поздно, чем рано. Уведомление называет очередь. | > 4 ч | 15 мин | Warning (`critical`) | 6 ч |
| Mail gateway service stopped | Остановлена служба, необходимая шлюзу для приёма и фильтрации почты (`dumbmonit_pmg_service_running` для postfix, pmg-smtp-filter, pmgpolicy, pmgproxy, pmgdaemon). Не установленный юнит не имеет ряда и не может сработать. | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| Virus signatures out of date | Ежедневным вирусным сигнатурам ClamAV более двух дней (`dumbmonit_pmg_signature_age_seconds{family="virus",database="daily"}`): freshclam перестал их обновлять, и шлюз продолжает фильтровать по прошлонедельному списку. | > 2 дн | 1 ч | Advisory (`warning`) | 24 ч |
| Spam rules out of date | Правилам SpamAssassin более восьми дней (`dumbmonit_pmg_signature_age_seconds{family="spam"}`). У канала, который никогда не обновлялся, нет ни даты, ни ряда. | > 8 дн | 1 ч | Advisory (`warning`) | 7 дн |
| Quarantine filling up | В спам-карантин поступило более тысячи сообщений за шесть часов (`delta(dumbmonit_pmg_quarantine_messages{kind="spam"}[6h])`): кампания или правило, только что отправившее легитимный домен в корзину. | > 1000 | 30 мин | Advisory (`warning`) | 12 ч |
| Mail gateway cluster degraded | Шлюз кластера больше не синхронизирован с остальными (`dumbmonit_pmg_cluster_node_insync`): он фильтрует по устаревшей базе правил. У автономного шлюза ряда нет. | < 1 | 15 мин | Advisory (`warning`) | 6 ч |
| Mail gateway certificate expiring | Сертификат почтового шлюза истекает менее чем через четырнадцать дней (`dumbmonit_pmg_certificate_expires_in_seconds`). | < 14 дн | 1 ч | Advisory (`warning`) | 24 ч |
| Mail gateway updates pending | На почтовом шлюзе ожидает более двадцати обновлений пакетов (`dumbmonit_pmg_node_updates_pending`). | > 20 | 1 ч | Info (`info`) | 7 дн |

### MDaemon и SecurityGateway {#mdaemon-and-securitygateway}

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| MDaemon mail service down | Отслеживаемая почтовая служба сервера MDaemon не отвечает или приветствует лишь затем, чтобы отказать в обслуживании (`dumbmonit_mdaemon_service_up`). Ряд есть только у служб, перечисленных на устройстве. Уведомление называет службу и её порт. | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| MDaemon XML API not answering | XML API перестал отвечать, пока почтовые службы ещё отвечают (`dumbmonit_mdaemon_api_up`): вероятно, остановлено Remote Administration. Ряд есть только у устройства с учётной записью. | < 1 | 15 мин | Advisory (`warning`) | 12 ч |
| MDaemon mail queue growing | Входящая, локальная или удалённая очередь выросла более чем на сто сообщений за два часа (`delta(dumbmonit_mdaemon_queue_messages{queue=~"inbound\|local\|remote"}[2h])`): MDaemon перестал доставлять из неё. Читается [агентом](../devices/agent.md#windows-performance-counters) на сервере MDaemon, поэтому оповещение относится к устройству агента. | > 100 | 15 мин | Advisory (`warning`) | 6 ч |
| MDaemon Bad queue not empty | В очереди Bad ждут сообщения (`dumbmonit_mdaemon_queue_messages{queue="bad"}`): MDaemon не смог ни доставить, ни вернуть их, и они остаются там, пока администратор не освободит или не удалит их. | > 0 | 30 мин | Advisory (`warning`) | 24 ч |
| MDaemon Retry queue high | Более пятидесяти сообщений ждут в очереди Retry (`dumbmonit_mdaemon_queue_messages{queue="retry"}`): удалённые серверы продолжают временно отказывать в приёме почты. | > 50 | 1 ч | Advisory (`warning`) | 6 ч |
| SecurityGateway service down | Отслеживаемая служба шлюза не отвечает или приветствует лишь затем, чтобы отказать в обслуживании (`dumbmonit_securitygateway_service_up`). | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| SecurityGateway API not answering | REST API перестал отвечать, пока службы ещё отвечают (`dumbmonit_securitygateway_api_up`). Ряд есть только у устройства с ключом API. | < 1 | 15 мин | Advisory (`warning`) | 12 ч |
| SecurityGateway delivery queue growing | Очередь доставки выросла более чем на сто сообщений за два часа (`delta(dumbmonit_securitygateway_counter{counter=~".*(delivery_queue\|queued_for_delivery).*"}[2h])`): почтовый сервер за шлюзом, вероятно, отказывается принимать почту. Имена счётчиков берутся из API как есть; если ваша версия называет очередь иначе, поправьте matcher. | > 100 | 15 мин | Advisory (`warning`) | 6 ч |

### OPNsense

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Internet gateway down | dpinger больше не получает ответа от шлюза (`dumbmonit_opnsense_gateway_up`). На межсетевом экране с несколькими WAN трафик уже тихо ушёл на другой канал. `loss` и `delay` — предупреждения, а не сбои, и это правило не вызывают. | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| Gateway losing packets | Шлюз теряет более пятой части отправленных ему пакетов (`dumbmonit_opnsense_gateway_loss_percent`): канал отвечает, но плохо. Сбрасывается ниже 10 %. | > 20 % | 10 мин | Advisory (`warning`) | 6 ч |
| Gateway slow | Время кругового обхода шлюза остаётся выше 300 мс (`dumbmonit_opnsense_gateway_delay_seconds`). Триста миллисекунд не трогают резервный мобильный канал. | > 300 мс | 15 мин | Info (`info`) | 12 ч |
| Firewall state table filling up | pf отслеживает более 80 % разрешённых ему соединений (`dumbmonit_opnsense_pf_states_used_percent`). За пределом межсетевой экран отбрасывает новые соединения без каких-либо других симптомов. | > 80 % | 10 мин | Advisory (`warning`) | 6 ч |
| Firewall network buffers exhausted | Используется более 90 % сетевых буферов FreeBSD (`dumbmonit_opnsense_mbuf_used_percent`). Межсетевой экран, у которого они закончились, перестаёт пересылать трафик при простаивающем CPU. | > 90 % | 10 мин | Advisory (`warning`) | 6 ч |
| VPN tunnel down | У настроенного VPN-туннеля нет сеанса в течение десяти минут (`dumbmonit_opnsense_vpn_tunnel_up`). Не установленный плагин или отключённое соединение не имеют ряда и не могут сработать. | < 1 | 10 мин | Advisory (`warning`) | 6 ч |
| Firewall resolver stopped | Unbound перестал отвечать (`dumbmonit_opnsense_unbound_running`). Межсетевой экран по-прежнему маршрутизирует; машины за ним больше не разрешают имена. Отключите это правило, если ваш межсетевой экран разрешает имена чем-то другим. | < 1 | 5 мин | Advisory (`warning`) | 6 ч |
| Firewall core service stopped | dpinger, следящий за шлюзами, или configd, запускающий всё остальное, остановлен (`dumbmonit_opnsense_service_running{service=~"dpinger\|configd"}`). Только эти два, потому что намеренно выключенная служба — не сбой. | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| Firewall too hot | Датчик показывает более 85 °C (`dumbmonit_opnsense_temperature_celsius`): безвентиляторная коробка в шкафу вот-вот начнёт троттлить. Сбрасывается при 78 °C. | > 85 °C | 10 мин | Advisory (`warning`) | 6 ч |
| Firewall left in CARP maintenance mode | CARP оставлен в постоянном режиме обслуживания (`dumbmonit_opnsense_carp_maintenance_mode`): этот межсетевой экран передал свои виртуальные адреса партнёру и сам их не заберёт обратно. | > 0 | 1 ч | Info (`info`) | 24 ч |
| Firewall reboot pending | Обновление установлено, но для вступления в силу требуется перезагрузка (`dumbmonit_opnsense_firmware_reboot_required`). | > 0 | 1 ч | Info (`info`) | 7 дн |
| Firewall updates pending | У межсетевого экрана есть пакеты, ожидающие обновления (`dumbmonit_opnsense_firmware_updates_pending`). Это самая уязвимая машина в сети. | > 0 | 1 ч | Info (`info`) | 7 дн |

### TrueNAS

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| NAS pool degraded | ZFS больше не считает пул здоровым (`dumbmonit_truenas_pool_healthy`). Деградировавший пул всё ещё отдаёт свои данные, именно поэтому никто не замечает, пока не откажет следующий диск. Страница устройства называет диск. | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| NAS disk errors | Диски пула сообщили об ошибках чтения, записи или контрольной суммы с момента последнего `zpool clear` (`dumbmonit_truenas_pool_device_errors`, один ряд на вид). | > 0 | 15 мин | Advisory (`warning`) | 24 ч |
| NAS pool almost full | Пул заполнен более чем на 85 % (`dumbmonit_truenas_pool_used_percent`). ZFS замедляется задолго до заполнения. Сбрасывается при 80 %. | > 85 % | 30 мин | Advisory (`warning`), эскалируется через 7 дн | 24 ч |
| NAS scrub found errors | Последний scrub пула нашёл повреждённые данные (`dumbmonit_truenas_pool_last_scrub_errors`). | > 0 | 5 мин | Warning (`critical`) | 24 ч |
| NAS scrub overdue | Пул не завершал scrub более 45 дней (`dumbmonit_truenas_pool_last_scrub_age_seconds`). Отсутствует после resilver, который стирает запись о последнем scrub. | > 45 дн | 1 ч | Info (`info`) | 7 дн |
| NAS disk failed its SMART test | Самотест SMART в журнале диска завершился неудачей (`dumbmonit_truenas_disk_smart_failed`). У диска, который никогда не тестировался, ряда нет. | > 0 | 5 мин | Warning (`critical`) | 24 ч |
| NAS disk too hot | Диск выше 55 °C (`dumbmonit_truenas_disk_temperature_celsius`, читается из кэша TrueNAS). Сбрасывается при 50 °C. | > 55 °C | 15 мин | Advisory (`warning`) | 6 ч |
| NAS dataset near its quota | Датасет использовал более 90 % своей квоты (`dumbmonit_truenas_dataset_quota_used_percent`): при 100 % записи в него завершаются ошибкой, хотя в пуле ещё есть место. Нет квоты — нет ряда. | > 90 % | 30 мин | Advisory (`warning`) | 24 ч |
| NAS replication failed | Включённая задача репликации в состоянии ошибки (`dumbmonit_truenas_replication_error`): копия на другой стороне стареет. | > 0 | 10 мин | Advisory (`warning`) | 12 ч |
| NAS snapshot task failed | Включённая задача периодических снимков в состоянии ошибки (`dumbmonit_truenas_snapshot_task_error`). | > 0 | 10 мин | Advisory (`warning`) | 12 ч |
| NAS snapshots stale | Включённая задача периодических снимков не запускалась более восьми дней (`dumbmonit_truenas_snapshot_task_last_run_age_seconds`): задача, которая перестаёт запускаться, не завершается ошибкой, она просто замолкает. | > 8 дн | 1 ч | Advisory (`warning`) | 24 ч |
| TrueNAS alert raised | Сам TrueNAS поднял оповещение уровня ERROR или выше (`dumbmonit_truenas_alerts`). Его собственная формулировка — на странице устройства. | > 0 | 10 мин | Advisory (`warning`) | 12 ч |
| NAS service stopped | Служба, настроенная на запуск вместе с NAS, остановлена (`dumbmonit_truenas_service_running`). У намеренно выключенной службы ряда нет. | < 1 | 10 мин | Advisory (`warning`) | 6 ч |

### Synology DSM

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Synology disk SMART warning | DSM больше не сообщает состояние SMART этого диска как `normal` (`dumbmonit_synology_disk_smart_status`: 1 attention, 2 critical, неизвестные слова считаются как 1). | > 0 | 15 мин | Advisory (`warning`) | 24 ч |
| Synology disk failed | DSM сообщает, что диск вышел из строя или неисправен (`dumbmonit_synology_disk_status`). | ≥ 2 | 5 мин | Warning (`critical`) | 6 ч |
| Synology disk bad sectors | Число плохих секторов диска превышает порог, заданный в DSM (`dumbmonit_synology_disk_bad_sector_exceeded`). | > 0 | 15 мин | Warning (`critical`) | 24 ч |
| Synology disk bad sectors growing | За последние 24 часа появились новые нечитаемые секторы (`delta(dumbmonit_synology_disk_unc_count[24h])`). | > 0 | 15 мин | Advisory (`warning`) | 24 ч |
| Synology SSD wearing out | У SSD осталось менее 10 % номинального ресурса (`dumbmonit_synology_disk_remaining_life_percent`); сбрасывается выше 12 %. | < 10 % | 1 ч | Advisory (`warning`) | 7 дн |
| Synology volume degraded | Том деградировал или вышел из строя: избыточность утрачена (`dumbmonit_synology_volume_status`). | ≥ 2 | 5 мин | Warning (`critical`) | 6 ч |
| Synology storage pool degraded | Пул хранения или SSD-кэш деградировал или вышел из строя: диск отказал, избыточность утрачена или перестраивается (`dumbmonit_synology_pool_status or dumbmonit_synology_ssd_cache_status`). | ≥ 2 | 5 мин | Warning (`critical`) | 6 ч |
| Synology volume almost full | Том заполнен на 90 % или более (`dumbmonit_synology_volume_used_percent`); сбрасывается ниже 88 %. | ≥ 90 % | 15 мин | Advisory (`warning`), эскалируется через 24 ч | 6 ч |
| Synology temperature high | Диск выше 55 °C или DSM поднял собственное предупреждение о температуре (`dumbmonit_synology_disk_temperature_celsius`, `dumbmonit_synology_temperature_warning`); сбрасывается ниже 52 °C. | > 55 °C | 15 мин | Advisory (`warning`) | 6 ч |
| Synology memory high | Использование памяти NAS выше 95 % (`dumbmonit_synology_memory_usage_percent`); сбрасывается ниже 90 %. | > 95 % | 15 мин | Advisory (`warning`) | 6 ч |
| Active Backup task failed | Последний запуск задачи Active Backup for Business завершился неудачей или скопировал лишь часть своих устройств (`dumbmonit_abb_task_last_status == bool 0`). Уведомление называет задачу. | > 0 | 10 мин | Warning (`critical`) | 24 ч |
| Active Backup too old | Нет успешного запуска Active Backup for Business для этой задачи более двух дней (`dumbmonit_abb_task_last_success_seconds`). Ряда нет для задачи, которая ни разу не завершалась успешно; тогда говорит "Active Backup task failed". | > 2 дн | 1 ч | Advisory (`warning`) | 24 ч |
| Active Backup task disabled | У задачи нет расписания или её непрерывное резервное копирование приостановлено (`dumbmonit_abb_task_enabled == bool 0`): она ничего не скопирует, пока кто-нибудь её не запустит. | > 0 | 1 ч | Info (`info`) | 7 дн |
| Active Backup device overdue | Устройство дольше без успешного запуска Active Backup for Business, чем позволяет его собственный ритм, исключая выходные дни (`dumbmonit_abb_device_overdue`, см. [Synology](../devices/synology.md#how-overdue-is-judged)). | > 0 | 30 мин | Advisory (`warning`) | 24 ч |
| Active Backup device failing | Последние две или более попытки устройства завершились неудачей (`dumbmonit_abb_device_consecutive_failures`); отменённый запуск не считается ни тем, ни другим. | ≥ 2 | 10 мин | Warning (`critical`) | 24 ч |

### Серверы логов и метрик {#log-and-metrics-servers}

VictoriaMetrics, VictoriaLogs, Loki и Graylog ([устройства](../devices/victoriametrics.md)). Счётчики проходят через `increase_prometheus`, поэтому сервер, добавленный с историей отклонённых данных, не срабатывает на первом же измерении.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Metrics or log storage read-only | Свободное место на диске упало ниже `-storage.minFreeDiskSpaceBytes`, и хранилище отклоняет любую запись (`dumbmonit_victoriametrics_read_only or dumbmonit_victorialogs_read_only`). | ≥ 1 | 2 мин | Warning (`critical`) | 1 ч |
| Metrics or log storage almost read-only | Осталось менее 10% диска до перехода в режим только для чтения (`dumbmonit_victoriametrics_disk_headroom_percent or dumbmonit_victorialogs_disk_headroom_percent`); сбрасывается выше 12 %. | < 10 % | 30 мин | Advisory (`warning`) | 24 ч |
| Metrics or log server unhealthy | `/health` не отвечает OK (`dumbmonit_victoriametrics_healthy or dumbmonit_victorialogs_healthy`). | < 1 | 3 мин | Warning (`critical`) | 6 ч |
| Metrics or log data refused | Сэмплы или строки логов отклонены за последний час: метки времени вне срока хранения, слишком длинные метки, лимит рядов (`increase_prometheus(dumbmonit_victoriametrics_rows_rejected_total[1h]) or increase_prometheus(dumbmonit_victorialogs_rows_rejected_total[1h])`). | > 0 | 15 мин | Advisory (`warning`) | 24 ч |
| VictoriaMetrics slow inserts | Более 5% новых сэмплов за 15 минут пошли по медленному пути: не хватает памяти для активных рядов (`(100 * increase_prometheus(dumbmonit_victoriametrics_slow_inserts_total[15m]) / increase_prometheus(dumbmonit_victoriametrics_rows_added_total[15m])) and on (target) dumbmonit_victoriametrics_uptime_seconds > 900`); сбрасывается ниже 3 %. Оценивается, только когда сервер проработал 15 минут, поэтому перезапуск (холодный кэш, счётчики обнулились внутри окна) не оставляет правило в ожидании. | > 5 % | 30 мин | Advisory (`warning`) | 24 ч |
| Loki not ready | `/ready` не отвечает ready (`dumbmonit_loki_ready`). | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| Loki refusing log lines | Строки отклонены за последний час: слишком старые, лимит скорости или потока, слишком длинная строка (`increase_prometheus(dumbmonit_loki_discarded_lines_total[1h])`). | > 0 | 15 мин | Advisory (`warning`) | 24 ч |
| Loki cannot flush to storage | Чанки не смогли попасть в хранилище за последние 30 минут; они теряются, если Loki перезапустится (`increase_prometheus(dumbmonit_loki_flush_failures_total[30m])`). | > 0 | 10 мин | Warning (`critical`) | 1 ч |
| Loki write-ahead log disk full | Записи WAL потеряны из-за заполненного диска (`increase_prometheus(dumbmonit_loki_wal_disk_full_failures_total[30m])`). | > 0 | 5 мин | Warning (`critical`) | 1 ч |
| Loki request errors | Более пяти запросов получили ответ с ошибкой сервера за 15 минут (`increase_prometheus(dumbmonit_loki_request_errors_total[15m])`). | > 5 | 15 мин | Advisory (`warning`) | 6 ч |
| Graylog not processing | Обработка сообщений приостановлена на узле (`dumbmonit_graylog_processing`). | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| Graylog search cluster down | OpenSearch или Elasticsearch в состоянии red или недоступен из Graylog (`dumbmonit_graylog_indexer_status`: 2 red, 3 недоступен). | ≥ 2 | 5 мин | Warning (`critical`) | 1 ч |
| Graylog search cluster yellow | Реплики шардов не назначены в течение получаса (`dumbmonit_graylog_indexer_status == 1`). | > 0 | 30 мин | Advisory (`warning`) | 24 ч |
| Graylog journal filling | Журнал заполнен более чем наполовину: вывод не справляется (`dumbmonit_graylog_journal_used_percent`); сбрасывается ниже 40 %. | > 50 % | 10 мин | Advisory (`warning`), эскалируется через 1 ч | 6 ч |
| Graylog input failed | Не удалось запустить input (`dumbmonit_graylog_inputs_failed`). | > 0 | 5 мин | Advisory (`warning`) | 24 ч |
| Graylog indexing failures | Записи в кластер поиска или обработка сообщений завершились неудачей за последний час (`increase_prometheus(dumbmonit_graylog_output_failures_total[1h]) + increase_prometheus(dumbmonit_graylog_processing_failures_total[1h])`). | > 0 | 15 мин | Advisory (`warning`) | 24 ч |

### Самостоятельно размещаемые приложения {#self-hosted-applications}

Nextcloud, Immich, Paperless-ngx, Jellyfin и Plex ([устройства](../devices/nextcloud.md)). Они редко падают полностью: страница открывается, а синхронизация, миниатюры, импорт или сканирование библиотек остановились. Эти правила следят за такими состояниями; приложение, которое не отвечает вовсе, — это Device unreachable.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Nextcloud stuck in maintenance | Режим обслуживания включён (`dumbmonit_nextcloud_maintenance`): никто не может войти или синхронизироваться. | > 0 | 30 мин | Advisory (`warning`), эскалируется через 3 ч | 6 ч |
| Nextcloud database upgrade pending | Код обновлён, и Nextcloud ждёт `occ upgrade` (`dumbmonit_nextcloud_needs_db_upgrade`). | > 0 | 10 мин | Warning (`critical`) | 1 ч |
| Nextcloud update available | Проверка обновлений Nextcloud нашла более новый выпуск (`dumbmonit_nextcloud_update_available`). | > 0 | 1 ч | Info (`info`) | 7 дн |
| Nextcloud OPcache full | В OPcache PHP закончилось место, и страницы замедляются (`dumbmonit_nextcloud_opcache_full`). | > 0 | 30 мин | Advisory (`warning`) | 24 ч |
| Nextcloud free space low | Менее 5 ГиБ свободно для каталога данных (`dumbmonit_nextcloud_free_space_bytes / 1073741824`); сбрасывается выше 6 ГиБ. | < 5 ГиБ | 30 мин | Advisory (`warning`) | 24 ч |
| Immich jobs stalled | Задания ждут в очереди, которая не приостановлена и ничего не выполняет (`dumbmonit_immich_queue_waiting > 0 and on (target, queue) dumbmonit_immich_queue_active == 0 unless on (target, queue) dumbmonit_immich_queue_paused == 1`). Уведомление называет очередь. | > 0 | 1 ч | Advisory (`warning`) | 6 ч |
| Immich queue paused | Очередь заданий приостановлена в течение суток (`dumbmonit_immich_queue_paused`). | > 0 | 24 ч | Info (`info`) | 7 дн |
| Immich storage almost full | Диск с библиотекой заполнен более чем на 90 % (`dumbmonit_immich_storage_used_percent`); сбрасывается ниже 88 %. | > 90 % | 30 мин | Advisory (`warning`) | 24 ч |
| Immich update available | Проверка версий Immich знает более новый выпуск (`dumbmonit_immich_update_available`). | > 0 | 1 ч | Info (`info`) | 7 дн |
| Paperless task queue down | Redis или Celery в состоянии ошибки (`max by (target) (dumbmonit_paperless_component_status{component=~"redis\|celery"})`: 2 — это ошибка): ничего нового не импортируется. | ≥ 2 | 5 мин | Warning (`critical`) | 1 ч |
| Paperless index or sanity check error | Поисковый индекс, классификатор или проверка целостности (отсутствующие или изменённые файлы) сообщают об ошибке (`dumbmonit_paperless_component_status{component=~"index\|classifier\|sanity_check"}`). | ≥ 2 | 30 мин | Advisory (`warning`) | 24 ч |
| Paperless migrations pending | Остались неприменённые миграции базы данных (`dumbmonit_paperless_unapplied_migrations`). | > 0 | 30 мин | Advisory (`warning`) | 24 ч |
| Paperless task failed | Задача, видимая этому пользователю, завершилась неудачей в окне неудачных задач и не была отклонена (`dumbmonit_paperless_tasks_failed_recent`). | > 0 | 5 мин | Advisory (`warning`) | 24 ч |
| Paperless storage almost full | Диск с документами заполнен более чем на 90 % (`dumbmonit_paperless_storage_used_percent`); сбрасывается ниже 88 %. | > 90 % | 30 мин | Advisory (`warning`) | 24 ч |
| Paperless update available | Проверка версий Paperless-ngx знает более новый выпуск (`dumbmonit_paperless_update_available`). | > 0 | 1 ч | Info (`info`) | 7 дн |
| Jellyfin scheduled task failed | Плановая задача (сканирование библиотеки, очистка) завершилась неудачей или была прервана при последнем запуске (`dumbmonit_jellyfin_scheduled_tasks_failed`). | > 0 | 10 мин | Advisory (`warning`) | 24 ч |
| Jellyfin plugin broken | Плагин не удалось загрузить или он не поддерживает эту версию (`dumbmonit_jellyfin_plugins_broken`). | > 0 | 15 мин | Advisory (`warning`) | 24 ч |
| Jellyfin restart pending | Jellyfin ждёт перезапуска сутки после установки или обновления плагина (`dumbmonit_jellyfin_pending_restart`). | > 0 | 24 ч | Info (`info`) | 7 дн |
| Plex update available | Проверка обновлений Plex нашла более новый выпуск (`dumbmonit_plex_update_available`). | > 0 | 1 ч | Info (`info`) | 7 дн |

### Самостоятельно размещаемые forge {#self-hosted-forges}

GitLab (self-managed) и Forgejo/Gitea ([устройства](../devices/gitlab.md)).
Экземпляр, который не отвечает вовсе, — это Device unreachable; эти правила
следят за частями, которые продолжают отдавать страницы, тихо отставая, — Sidekiq,
раннеры CI/CD, плановая задача, которая перестала запускаться.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| GitLab runners all offline | Все раннеры CI/CD, известные GitLab, офлайн, и зарегистрирован хотя бы один (`dumbmonit_gitlab_runners_online and on (target) dumbmonit_gitlab_runners_total > 0`). | < 1 | 10 мин | Advisory (`warning`) | 6 ч |
| GitLab Sidekiq backlog high | В очереди Sidekiq ожидает более 500 заданий (`max by (target) (dumbmonit_gitlab_sidekiq_queue_backlog)`): уведомления, вебхуки и импорты отстают. | > 500 | 15 мин | Advisory (`warning`) | 6 ч |
| GitLab migrations pending | Остались неприменённые миграции базы данных после обновления (`dumbmonit_gitlab_migrations_pending`). | > 0 | 1 ч | Info (`info`) | 24 ч |
| GitLab license expiring | Лицензия Enterprise Edition истекает менее чем через 30 дней (`dumbmonit_gitlab_license_expiry_seconds`). | < 30 дн | 1 ч | Advisory (`warning`) | 24 ч |
| Forgejo/Gitea runners all offline | Все раннеры Actions офлайн, и зарегистрирован хотя бы один (`dumbmonit_forgejo_runners_online and on (target) dumbmonit_forgejo_runners_total > 0`). | < 1 | 10 мин | Advisory (`warning`) | 6 ч |
| Forgejo/Gitea scheduled task overdue | Плановая задача (проверка репозитория, очистка, проверка обновлений) не запустилась в свой интервал (`dumbmonit_forgejo_cron_task_overdue`). | > 0 | 30 мин | Advisory (`warning`) | 24 ч |

### Клиентские устройства {#client-devices}

Одно правило, общее для всех интеграций, отслеживающих подключающиеся и копирующиеся устройства — на сегодня это телефоны, планшеты и настольные компьютеры Immich ([устройства](../devices/immich.md)), копируемые машины Proxmox Backup Server, по одной на группу резервного копирования ([устройства](../devices/pbs.md)), и защищаемые компьютеры и ВМ Veeam, по одному на объект с точкой восстановления ([устройства](../devices/veeam.md)); со временем появятся другие. Каждая интеграция вычисляет отставание относительно окна устаревания своей цели (`device_stale_days`, по умолчанию 3 дня) и публикует его как `dumbmonit_client_device_stale_seconds` с метками `device`, `type`, `os`, `user`, `kind` и `signal` (`connection` или `backup`); затем это единственное правило срабатывает для любого из них, называя устройство в уведомлении. Synology Active Backup for Business отслеживает свои устройства отдельно, через механизм, учитывающий ритм (`synology_abb_device_*`), а не через это общее семейство — см. [Synology](../devices/synology.md).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Client device stale | Устройство не подключалось или не копировалось в пределах окна устаревания, настроенного для этой цели (`dumbmonit_client_device_stale_seconds`). | > 0 | 1 ч | Advisory (`warning`) | 24 ч |

### Pi-hole

[Pi-hole](../devices/pihole.md) v6, читается через его REST API.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Pi-hole blocking disabled | Блокировка выключена, приостановлена или не удалась (`dumbmonit_pihole_blocking_enabled`). Выдержка позволяет пережить пятиминутную паузу из веб-интерфейса. | < 1 | 15 мин | Advisory (`warning`) | 6 ч |
| Pi-hole blocklists stale | Списки блокировки (gravity) не пересобирались более восьми дней (`dumbmonit_pihole_gravity_age_seconds`): еженедельное обновление не удаётся. | > 8 дн | 1 ч | Advisory (`warning`) | 24 ч |
| Pi-hole update available | Доступна более новая версия (`dumbmonit_pihole_updates_available`): образ в Docker, а иначе core, веб-интерфейс или FTL. Pi-hole проверяет раз в сутки. | > 0 | 1 ч | Info (`info`) | 7 дн |
| Pi-hole diagnosis messages | Pi-hole перечисляет сообщения на странице диагностики (`dumbmonit_pihole_messages`): список блокировки, который не удалось загрузить, клиент с ограничением скорости, перегруженный хост. | > 0 | 15 мин | Info (`info`) | 24 ч |

### AdGuard Home

DNS-фильтр ([устройство](../devices/adguard.md)). Показатели запросов и блокировок — суммы за собственное окно статистики AdGuard Home, а не счётчики: на них не построено ни одно правило.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| AdGuard Home protection off | Защита отключена или приостановлена и ещё не вернулась (`dumbmonit_adguard_protection_enabled`). | < 1 | 15 мин | Advisory (`warning`) | 6 ч |
| AdGuard Home DNS not running | Веб-интерфейс отвечает, но DNS-сервер остановлен (`dumbmonit_adguard_running`). | < 1 | 5 мин | Warning (`critical`) | 1 ч |
| AdGuard Home upstream failing | Вышестоящий DNS-сервер не проходит собственный тест AdGuard Home или молчит 4 секунды (`dumbmonit_adguard_upstream_up`, один ряд на upstream). | < 1 | 10 мин | Advisory (`warning`) | 6 ч |
| AdGuard Home filter lists stale | Самому старому обновлению включённого списка фильтров более трёх дней (`dumbmonit_adguard_filter_oldest_update_age_seconds`). | > 3 дней | 1 ч | Advisory (`warning`) | 24 ч |
| AdGuard Home update available | Собственная проверка версий AdGuard Home объявила более новую версию (`dumbmonit_adguard_update_available`); никогда не срабатывает, когда эта проверка отключена, как в официальном образе Docker. | > 0 | 1 ч | Info (`info`) | 7 дней |

### ИБП за NUT {#ups-behind-nut}

ИБП, читаемые с [сервера NUT](../devices/nut.md). Каждый ряд несёт метку `ups`,
поэтому оповещение называет ИБП, когда один сервер публикует несколько. Флаги
состояния при каждом чтении равны 0 или 1, поэтому правило сбрасывается само,
как только возвращается сетевое питание. ИБП, читаемые по SNMP, охватываются
правилами [ИБП](#ups) выше.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| UPS on battery (NUT) | ИБП питает нагрузку от батареи (`dumbmonit_nut_ups_on_battery`, флаг `OB` в `ups.status`). | > 0 | 30 с | Warning (`critical`) | 15 мин |
| UPS battery low (NUT) | ИБП сообщает о низком заряде батареи, флаг `LB` (`dumbmonit_nut_ups_low_battery`): питаемые им машины вот-вот выключатся. | > 0 | 1 мин | Warning (`critical`) | 30 мин |
| UPS battery needs replacing (NUT) | ИБП просит заменить батарею, флаг `RB` (`dumbmonit_nut_ups_replace_battery`). | > 0 | 1 ч | Advisory (`warning`) | 7 дн |
| UPS load high (NUT) | Нагрузка остаётся выше 80 % (`dumbmonit_nut_ups_load_percent`); сбрасывается ниже 75 %. | > 80 % | 10 мин | Advisory (`warning`) | 24 ч |
| UPS runtime short (NUT) | ИБП оценивает время работы от батареи менее пяти минут (`dumbmonit_nut_battery_runtime_seconds`); сбрасывается выше шести. При сетевом питании это изношенная батарея или ИБП, слишком маленький для своей нагрузки. | < 300 с | 5 мин | Advisory (`warning`) | 24 ч |
| UPS data stale (NUT) | Сервер NUT отвечает, но не имеет свежих данных от ИБП: кабель, драйвер или ИБП молчит (`dumbmonit_nut_ups_data_stale`). Тем временем отключение электричества осталось бы незамеченным. | > 0 | 5 мин | Advisory (`warning`), эскалируется через 1 ч | 6 ч |

### MikroTik RouterOS

Ряды несут метку `interface` или `sensor`, поэтому уведомление называет порт или датчик ([устройство](../devices/mikrotik.md)). Счётчики проходят через `increase_prometheus`: маршрутизатор, добавленный с годами ошибок за плечами, не срабатывает на первом же измерении.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| MikroTik CPU high | CPU маршрутизатора остаётся выше 90 % (`dumbmonit_mikrotik_cpu_load_percent`): его насыщают маршрутизация, межсетевой экран или скрипт. Сбрасывается ниже 80 %. | > 90 % | 15 мин | Advisory (`warning`) | 6 ч |
| MikroTik memory high | Используется более 90 % памяти (`dumbmonit_mikrotik_memory_used_percent`). Сбрасывается ниже 85 %. | > 90 % | 15 мин | Advisory (`warning`) | 6 ч |
| MikroTik storage almost full | Флеш-хранилище заполнено более чем на 90 % (`dumbmonit_mikrotik_storage_used_percent`): изменения конфигурации и обновления могут не удаваться. Сбрасывается ниже 85 %. | > 90 % | 30 мин | Advisory (`warning`) | 24 ч |
| MikroTik temperature high | Датчик температуры показывает выше 80 °C (`dumbmonit_mikrotik_temperature_celsius`). Сбрасывается при 75 °C. | > 80 °C | 10 мин | Advisory (`warning`) | 6 ч |
| MikroTik power supply or fan failed | Датчик `psu*-state` или `fan*-state` не в состоянии `ok` (`dumbmonit_mikrotik_health_ok`). Умерший резервный блок питания ничего не отключает, пока не умрёт и второй. | < 1 | 5 мин | Warning (`critical`) | 1 ч |
| MikroTik interface accumulating errors | Более пятидесяти ошибок приёма и передачи на интерфейсе за последний час (`increase_prometheus(dumbmonit_mikrotik_interface_rx_errors_total[1h]) + increase_prometheus(dumbmonit_mikrotik_interface_tx_errors_total[1h])`). | > 50 | 15 мин | Advisory (`warning`) | 24 ч |
| MikroTik link flapping | Интерфейс потерял линк более двух раз за тридцать минут (`increase_prometheus(dumbmonit_mikrotik_interface_link_downs_total[30m])`). Порт, отключённый навсегда, считается один раз и не срабатывает. | > 2 | 5 мин | Advisory (`warning`) | 1 ч |
| MikroTik RouterOS update available | На канале обновлений маршрутизатора доступна более новая RouterOS (`dumbmonit_mikrotik_update_available`). Известно только когда маршрутизатор сам проверяет обновления: см. необязательный шаг [настройки](../devices/mikrotik.md). | > 0 | 1 ч | Info (`info`) | 7 дн |
| MikroTik firmware upgrade pending | RouterBOOT старше прошивки, входящей в установленную RouterOS (`dumbmonit_mikrotik_firmware_upgrade_pending`); он обновляется только по запросу, затем при следующей перезагрузке. | > 0 | 1 ч | Info (`info`) | 7 дн |

### UniFi Network

[Страница устройства](../devices/unifi.md). Ряды устройств несут имя, модель и тип устройства, поэтому уведомление говорит «Living room AP». Устройство, ожидающее принятия, не имеет ряда `device_up` и никогда не считается офлайн.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| UniFi device offline | Устройство UniFi, принятое контроллером, офлайн, изолировано или не удалось принять (`dumbmonit_unifi_device_up`). | < 1 | 5 мин | Advisory (`warning`), эскалируется через 1 ч | 6 ч |
| UniFi Internet down | Шлюз UniFi сообщает об отсутствии подключения к Интернету (`dumbmonit_unifi_internet_up`). Без шлюза UniFi ряда не существует, и правило молчит. | < 1 | 3 мин | Warning (`critical`) | 1 ч |
| UniFi WAN link down | WAN-линк шлюза UniFi не работает (`dumbmonit_unifi_wan_link_up`): при двух линках резервирование утрачено, хотя ничего не кажется сломанным. У отключённого порта ряда нет. | < 1 | 5 мин | Advisory (`warning`) | 6 ч |
| UniFi gateway CPU high | Процессор шлюза UniFi выше 90% в течение пятнадцати минут (`dumbmonit_unifi_device_cpu_percent{type="gateway"}`); сбрасывается ниже 80 %. | > 90 % | 15 мин | Advisory (`warning`) | 6 ч |
| UniFi device waiting for adoption | В сети появилось устройство, которое ждёт принятия в UniFi (`dumbmonit_unifi_devices{state="pending"}`). | > 0 | 30 мин | Info (`info`) | 24 ч |
| UniFi firmware update available | Для устройств UniFi доступно обновление прошивки (`dumbmonit_unifi_devices_upgradable`). | > 0 | 1 ч | Info (`info`) | 7 дн |
| UniFi alarms | UniFi зафиксировал критические события или поднял тревоги за последние 24 часа (`dumbmonit_unifi_alarms`). Читается только с учётной записью View Only. | > 0 | 5 мин | Advisory (`warning`) | 24 ч |

### Home Assistant

[Страница устройства](../devices/homeassistant.md).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Home Assistant in recovery mode | Home Assistant запустился в режиме восстановления: его конфигурацию не удалось загрузить, и автоматизации не работают (`dumbmonit_homeassistant_recovery_mode`). | > 0 | 2 мин | Warning (`critical`) | 1 ч |
| Home Assistant not running | Home Assistant отвечает, но не завершил запуск в течение десяти минут (`dumbmonit_homeassistant_running`). | < 1 | 10 мин | Advisory (`warning`) | 6 ч |
| Home Assistant entities went unavailable | Недоступных сущностей как минимум на пять больше, чем час назад: остановилась интеграция, хаб или радио (`sum by (target) (dumbmonit_homeassistant_entities_unavailable) - sum by (target) (dumbmonit_homeassistant_entities_unavailable offset 1h)`). Считается прирост, а не общее число, которое в реальной установке никогда не равно нулю. | ≥ 5 | 10 мин | Advisory (`warning`) | 6 ч |
| Home Assistant battery low | Батарея, о которой сообщает Home Assistant, разряжена (`dumbmonit_homeassistant_battery_low`, один ряд на батарею ниже порога устройства, по умолчанию 20%; его значение — уровень заряда). | ≥ 0 % | 30 мин | Info (`info`) | 3 дн |
| Home Assistant update available | У Home Assistant есть ожидающие обновления: core, операционная система, дополнения или прошивка устройств (`dumbmonit_homeassistant_updates_available`). | > 0 | 1 ч | Info (`info`) | 7 дн |
| Home Assistant repair to address | В Home Assistant есть открытый ремонт с важностью error или critical (`dumbmonit_homeassistant_repair{severity=~"error\|critical"}`). | > 0 | 15 мин | Advisory (`warning`) | 24 ч |

### VMware vSphere

[Страница устройства](../devices/vsphere.md). Ряды хостов несут `esxi`, ряды ВМ — `vm`, ряды хранилищ данных — `datastore`, поэтому уведомление называет объект.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| vSphere host not responding | Хост vSphere не отвечает vCenter (`dumbmonit_vsphere_host_connection_state`: 2 не отвечает). Намеренно отключённый хост (1) не вызывает срабатывания. | ≥ 2 | 3 мин | Warning (`critical`) | 1 ч |
| vSphere host health red | vSphere сообщает общее состояние хоста как красное (`dumbmonit_vsphere_host_status == 2`). Серый означает неизвестно и не вызывает срабатывания. | > 0 | 5 мин | Warning (`critical`) | 1 ч |
| vSphere host left in maintenance | Хост vSphere находится в режиме обслуживания сутки: его ВМ перенесены, и о нём могли забыть (`dumbmonit_vsphere_host_maintenance`). | > 0 | 24 ч | Info (`info`) | 24 ч |
| vSphere datastore almost full | Хранилище данных vSphere заполнено на 90% или более (`dumbmonit_vsphere_datastore_used_percent`); сбрасывается ниже 85 %. | ≥ 90 % | 15 мин | Advisory (`warning`), эскалируется через 24 ч | 6 ч |
| vSphere datastore inaccessible | Хранилище данных vSphere недоступно: хранящиеся на нём ВМ не могут работать (`dumbmonit_vsphere_datastore_accessible`). | < 1 | 5 мин | Warning (`critical`) | 1 ч |
| vSphere VM tools not running | VMware Tools установлены, но не запущены во включённой ВМ: гость, возможно, завис (`dumbmonit_vsphere_vm_tools_status == 2`). ВМ без Tools не вызывают срабатывания. | > 0 | 30 мин | Info (`info`) | 24 ч |
| vSphere red alarm | vSphere поднял красную тревогу, которую никто не подтвердил (`dumbmonit_vsphere_alarm{status="red"}`, один ряд на тревогу, названный по тревоге и объекту). | > 0 | 5 мин | Advisory (`warning`) | 6 ч |

### Базы данных, брокер сообщений и движок безопасности {#databases-message-broker-and-security-engine}

Redis / Valkey, MongoDB, RabbitMQ и CrowdSec ([Redis](../devices/redis.md), [MongoDB](../devices/mongodb.md), [RabbitMQ](../devices/rabbitmq.md), [CrowdSec](../devices/crowdsec.md)). Счётчики проходят через `increase_prometheus` или `changes_prometheus`, поэтому ничего не срабатывает на первом измерении устройства.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Redis memory near maxmemory | Используемая память выше 90 % от `maxmemory` (`dumbmonit_redis_memory_used_percent`); сбрасывается ниже 85 %. Без `maxmemory` ряда нет. | > 90 % | 10 мин | Advisory (`warning`) | 6 ч |
| Redis refusing connections | Клиентские соединения отклонялись за последние 15 минут: достигнут `maxclients` (`increase_prometheus(dumbmonit_redis_rejected_connections_total[15m])`). | > 0 | 5 мин | Warning (`critical`) | 1 ч |
| Redis replication link down | Реплика потеряла связь с primary (`dumbmonit_redis_master_link_up`). | < 1 | 2 мин | Warning (`critical`) | 1 ч |
| Redis replica lagging | С точки зрения primary реплика не подтверждала более 30 секунд (`dumbmonit_redis_replica_lag_seconds`). | > 30 с | 5 мин | Advisory (`warning`) | 6 ч |
| Redis snapshot failed | Последний `BGSAVE` завершился неудачей; при `stop-writes-on-bgsave-error` запись отклоняется (`dumbmonit_redis_rdb_last_save_ok`). | < 1 | 5 мин | Warning (`critical`) | 1 ч |
| Redis append-only file write failed | Последняя запись AOF завершилась неудачей (`dumbmonit_redis_aof_last_write_ok`). | < 1 | 5 мин | Warning (`critical`) | 1 ч |
| MongoDB replica set without primary | Ни один участник не является primary: запись не принимается (`dumbmonit_mongodb_replset_primary_present`). | < 1 | 2 мин | Warning (`critical`) | 1 ч |
| MongoDB member unreachable | Участник набора реплик недоступен для остальных (`dumbmonit_mongodb_replset_member_health`). | < 1 | 5 мин | Warning (`critical`) | 1 ч |
| MongoDB replication lag | Secondary отстаёт от primary более чем на минуту (`dumbmonit_mongodb_replset_member_lag_seconds`); сбрасывается ниже 30 с. | > 60 с | 10 мин | Advisory (`warning`) | 6 ч |
| MongoDB connections near the limit | Используется более 80 % доступных соединений (`dumbmonit_mongodb_connections_used_percent`); сбрасывается ниже 75 %. | > 80 % | 10 мин | Advisory (`warning`) | 6 ч |
| MongoDB cache under pressure | Более 20 % кэша WiredTiger «грязные» (`dumbmonit_mongodb_wiredtiger_cache_dirty_percent`); сбрасывается ниже 15 %. | > 20 % | 10 мин | Advisory (`warning`) | 6 ч |
| MongoDB internal errors | Обычные assertions за последний час (`increase_prometheus(dumbmonit_mongodb_asserts_total{type="regular"}[1h])`). | > 0 | 15 мин | Advisory (`warning`) | 24 ч |
| RabbitMQ memory or disk alarm | Узел поднял аварию ресурса: все издатели заблокированы (`dumbmonit_rabbitmq_alarm`, один ряд на узел и ресурс). | > 0 | 1 мин | Warning (`critical`) | 1 ч |
| RabbitMQ node down | Узел кластера не работает (`dumbmonit_rabbitmq_node_running`). | < 1 | 2 мин | Warning (`critical`) | 1 ч |
| RabbitMQ network partition | Узел видит сетевое разделение (`dumbmonit_rabbitmq_node_partitions`). | > 0 | 2 мин | Warning (`critical`) | 1 ч |
| RabbitMQ queue without consumer | В очереди есть сообщения, а потребителя нет (`dumbmonit_rabbitmq_queue_messages_ready and dumbmonit_rabbitmq_queue_consumers == 0`); значение — число ожидающих сообщений. | > 0 | 15 мин | Advisory (`warning`) | 6 ч |
| RabbitMQ queue unavailable | Очередь не работает: quorum-очередь без большинства или её узел не работает (`dumbmonit_rabbitmq_queue_running`). | < 1 | 5 мин | Advisory (`warning`) | 6 ч |
| CrowdSec Local API down | Local API не отвечает на `/health` (`dumbmonit_crowdsec_lapi_up`). | < 1 | 5 мин | Warning (`critical`) | 1 ч |
| CrowdSec bouncer stopped pulling | Запросы bouncer не менялись за 30 минут (`changes_prometheus(dumbmonit_crowdsec_bouncer_requests_total[30m])`). | < 1 | 5 мин | Advisory (`warning`) | 6 ч |
| CrowdSec reads no logs | Ни одной строки лога не прочитано за шесть часов (`increase_prometheus(dumbmonit_crowdsec_lines_read_total[6h])`). | < 1 | 30 мин | Advisory (`warning`) | 24 ч |

### Обратные прокси и домены {#reverse-proxies-and-domains}

Traefik, Caddy, Nginx Proxy Manager и регистрации доменов ([Traefik](../devices/traefik.md), [Caddy](../devices/caddy.md), [Nginx Proxy Manager](../devices/npm.md), [Domain expiry](../devices/domain.md)), а также параметр DNS-проверки *Alert when the answer changes* ([Services](../devices/services.md#alerting-when-the-answer-changes)). Ряды несут `router`, `service`, `server`, `upstream`, `proxy_host` или `cert`, поэтому уведомление называет то, что вышло из строя.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Traefik router disabled | Маршрутизатор отключён из-за ошибки конфигурации (отсутствует сервис или middleware, недопустимое правило): его сайты не обслуживаются (`dumbmonit_traefik_router_status`: 0 включён, 1 предупреждение, 2 отключён). | ≥ 2 | 5 мин | Advisory (`warning`) | 6 ч |
| Traefik backend server down | Сервер сервиса с балансировкой нагрузки не проходит проверку работоспособности (`dumbmonit_traefik_server_up`). | < 1 | 5 мин | Advisory (`warning`) | 6 ч |
| Traefik service without a server | Ни один сервер сервиса не отвечает на проверку работоспособности: его сайты получают 503 (`dumbmonit_traefik_service_servers_up`). | < 1 | 2 мин | Warning (`critical`) | 1 ч |
| Traefik certificate not obtained | Маршрутизаторы запрашивают у распознавателя сертификатов сертификат, который пока не покрывает ни один сертификат, хранимый Traefik: Traefik отдаёт свой самоподписанный сертификат по умолчанию (`dumbmonit_traefik_resolver_routers_uncovered`, требуется `/api/certificates` Traefik). | > 0 | 1 ч | Advisory (`warning`) | 24 ч |
| Caddy upstream unhealthy | Бэкенд не проходит активную или пассивную проверку работоспособности Caddy (`dumbmonit_caddy_upstream_healthy`). | < 1 | 5 мин | Advisory (`warning`) | 6 ч |
| Caddy configuration reload failed | Caddy отклонил последнюю перезагрузку конфигурации и продолжает работать с предыдущей (`dumbmonit_caddy_config_last_reload_ok`). | < 1 | 5 мин | Advisory (`warning`) | 24 ч |
| Nginx Proxy Manager host offline | nginx отклонил конфигурацию включённого хоста: хост не обслуживается (`dumbmonit_npm_host_online`). | < 1 | 10 мин | Advisory (`warning`) | 6 ч |
| Proxy certificate not renewed | Сертификат, который хранит Traefik или предъявляет хост Nginx Proxy Manager, истекает менее чем через 14 дней: автоматическое продление начинается за 30, значит, оно не удаётся (`dumbmonit_traefik_cert_expiry_days or dumbmonit_npm_cert_expiry_days`). | < 14 дн | 1 ч | Advisory (`warning`) | 24 ч |
| Reverse proxy answering 5xx | Более 5 % ответов за десять минут были 5xx, при не менее чем пятидесяти запросах (`increase_prometheus` от `dumbmonit_{traefik,caddy}_requests_5xx_total` относительно `…_requests_total`). Требуются метрики прокси. | > 5 % | 10 мин | Advisory (`warning`) | 6 ч |
| Domain expiring | Домен истекает менее чем через 30 дней (`dumbmonit_domain_expiry_days >= 7`). | < 30 дн | 1 ч | Advisory (`warning`) | 24 ч |
| Domain about to expire | Домен истекает менее чем через 7 дней или уже истёк (`dumbmonit_domain_expiry_days`). | < 7 дн | 1 ч | Warning (`critical`) | 12 ч |
| Domain on hold | Реестр перевёл домен в `clientHold` или `serverHold`: он исключён из DNS (`dumbmonit_domain_on_hold`). | > 0 | 30 мин | Warning (`critical`) | 6 ч |
| Domain in redemption | Домен в состоянии `redemptionPeriod`, `pendingDelete` или `pendingRestore`: он истёк (`dumbmonit_domain_redemption`). | > 0 | 30 мин | Warning (`critical`) | 12 ч |
| DNS answer changed | Ответ DNS-проверки с *Alert when the answer changes* отличается от предыдущего (`changes_prometheus(dumbmonit_probe_dns_answer_fingerprint[1h])`). Разрешается само через час. | > 0 | 1 мин | Advisory (`warning`) | 24 ч |

### Изменения веб-сайтов {#website-changes}

См. [Website changes](../devices/webchange.md). Страница устройства показывает каждое изменение до и после.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Website changed | Проверка отслеживаемой страницы или сайта нашла страницы, которые изменились, появились или исчезли (`dumbmonit_webchange_last_check_changes`). О каждой такой проверке уведомляют с перечислением страниц, в рамках того же оповещения, пока оно активно; оно снимается при первой проверке, не нашедшей ничего нового. | > 0 | 30 с | Info (`info`) | нет |

### Kubernetes

См. [Kubernetes / k3s](../devices/kubernetes.md). Каждый ряд помечен узлом или
пространством имён и подом, рабочей нагрузкой или запросом тома, о которых идёт
речь.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Kubernetes node not ready | Узел не Ready: его kubelet больше не сообщает, и его поды будут вытеснены (`dumbmonit_k8s_node_ready == bool 0`). | > 0 | 5 мин | Warning (`critical`) | 6 ч |
| Kubernetes node under pressure | Узел сообщает о нехватке памяти, диска или PID (`dumbmonit_k8s_node_pressure`, метка `condition`). | > 0 | 5 мин | Advisory (`warning`) | 6 ч |
| Kubernetes pod crash looping | Контейнер в состоянии `CrashLoopBackOff` и всё ещё перезапускается: не менее двух перезапусков за 15 минут (`max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) and on (target, namespace, pod) increase_prometheus(dumbmonit_k8s_pod_restarts[15m]) >= 2`; состояние на мгновение исчезает при каждой попытке перезапуска). Контейнеры, убитые перезапуском узла или среды выполнения (код выхода 137, 143 или 255, не `OOMKilled`), не учитываются, поэтому перезагрузка не выглядит как цикл сбоев. | > 0 | 10 мин | Warning (`critical`) | 6 ч |
| Kubernetes pod stuck pending | Под находится в Pending пятнадцать минут: ни один узел не может его принять или не появляется необходимый ему том или образ (`dumbmonit_k8s_pod_pending`). | > 0 | 15 мин | Advisory (`warning`) | 6 ч |
| Kubernetes pod restarting | Под перезапускался более пяти раз за последний час, не находясь в `CrashLoopBackOff` (`increase_prometheus(dumbmonit_k8s_pod_restarts[1h]) unless on (target, namespace, pod) max_over_time(dumbmonit_k8s_pod_crashlooping[10m]) > 0`). | > 5 | 5 мин | Advisory (`warning`) | 6 ч |
| Kubernetes workload missing replicas | У Deployment, StatefulSet или DaemonSet меньше доступных реплик, чем он запрашивает (`dumbmonit_k8s_workload_unavailable`). | > 0 | 10 мин | Advisory (`warning`), эскалируется через 1 ч | 6 ч |
| Kubernetes volume claim pending | PersistentVolumeClaim, нужный поду или связанный с ошибкой provisioning, всё ещё не привязан (`dumbmonit_k8s_pvc_pending`). | > 0 | 15 мин | Advisory (`warning`) | 6 ч |

### Active Directory

См. [Active Directory](../devices/activedirectory.md). Контроллер домена,
который опрашивает DumbMonit, охватывается правилом *Device unreachable*; прочие
контроллеры домена — правилом *Domain controller unreachable* (метка `dc`).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Domain controller unreachable | Контроллер домена больше не отвечает на своём порту LDAP (`dumbmonit_ad_dc_reachable == bool 0`). Имя, которое не разрешается из DumbMonit, не даёт ряда и никогда не срабатывает. | > 0 | 5 мин | Warning (`critical`) | 6 ч |
| Active Directory bind failed | Контроллер домена отвечает, но отклоняет служебную учётную запись: истёкший пароль, отключённая или заблокированная учётная запись (`dumbmonit_ad_bind_ok == bool 0`). | > 0 | 5 мин | Advisory (`warning`) | 24 ч |
| Privileged group membership changed | Участник вошёл в Domain Admins, Enterprise Admins, Schema Admins, Administrators или группу операторов либо вышел из неё, включая вложенность (`changes_prometheus(dumbmonit_ad_privileged_group_fingerprint[1h])`, метка `group`). Разрешается само через час. | > 0 | 1 мин | Advisory (`warning`) | 24 ч |
| Active Directory replication failing | Опрашиваемый контроллер домена не может реплицироваться с партнёром (`dumbmonit_ad_replication_consecutive_failures`). Только когда учётная запись может читать состояние репликации. | > 0 | 30 мин | Advisory (`warning`) | 6 ч |
| krbtgt password older than 180 days | Пароль krbtgt последний раз менялся более 180 дней назад (`dumbmonit_ad_krbtgt_password_age_seconds`). | > 180 дней | 10 мин | Info (`info`) | нет |

### pfSense

См. [pfSense](../devices/pfsense.md). Шлюзы помечены `gateway`, интерфейсы — `interface` и `descr`, службы — `service`.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| pfSense gateway down | Шлюз pfSense не работает: его адрес мониторинга больше не отвечает, и несомый им трафик прекращается или переключается (`dumbmonit_pfsense_gateway_up`). | < 1 | 3 мин | Warning (`critical`) | 6 ч |
| pfSense gateway losing packets | Шлюз pfSense теряет более 10 % зондов к своему адресу мониторинга: линия деградировала (`dumbmonit_pfsense_gateway_loss_percent unless on (target, gateway) dumbmonit_pfsense_gateway_up == 0`). | > 10 % | 10 мин | Advisory (`warning`) | 6 ч |
| pfSense gateway slow | Время кругового обхода до адреса мониторинга шлюза pfSense остаётся выше 500 мс пятнадцать минут (`dumbmonit_pfsense_gateway_delay_milliseconds`). | > 500 мс | 15 мин | Advisory (`warning`) | 6 ч |
| pfSense interface without link | У включённого интерфейса pfSense нет линка: кабель, порт коммутатора или сетевая карта (`dumbmonit_pfsense_interface_down`). | > 0 | 5 мин | Advisory (`warning`) | 6 ч |
| pfSense service stopped | Включённая служба pfSense не запущена: DNS-резолвер, VPN, NTP… (`dumbmonit_pfsense_service_stopped`). | > 0 | 5 мин | Advisory (`warning`) | 6 ч |
| pfSense disk nearly full | Диск pfSense заполнен более чем на 90 %: логи, отчёты и обновления пакетов перестают записываться (`dumbmonit_pfsense_disk_used_percent`). | > 90 % | 30 мин | Advisory (`warning`) | 1 дн |

### Unraid

См. [Unraid](../devices/unraid.md). Диски помечены `disk` и `role` (`parity`, `data`, `cache`), контейнеры — `container`.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Unraid array stopped | Массив Unraid не запущен: шары, контейнеры и ВМ, расположенные на нём, недоступны (`dumbmonit_unraid_array_started`). | < 1 | 10 мин | Warning (`critical`) | 6 ч |
| Unraid disk disabled or missing | Диск Unraid отключён, отсутствует или недействителен: массив работает в деградированном режиме, и ещё один отказ приведёт к потере данных (`dumbmonit_unraid_disk_ok`). | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| Unraid disk read errors | Диск Unraid сообщал об ошибках чтения с момента запуска массива: при следующей он может быть отключён (`dumbmonit_unraid_disk_errors`). | > 0 | 5 мин | Advisory (`warning`) | 1 дн |
| Unraid disk too hot | Диск массива или чётности Unraid остаётся выше 55 °C пятнадцать минут (`dumbmonit_unraid_disk_temperature_celsius{role=~"data\|parity"}`). | > 55 °C | 15 мин | Advisory (`warning`) | 6 ч |
| Unraid parity check found errors | Последняя проверка чётности Unraid не удалась или нашла ошибки синхронизации: чётность больше не соответствует данным (`dumbmonit_unraid_parity_check_ok`). | < 1 | 5 мин | Advisory (`warning`) | 1 дн |
| Unraid parity check overdue | Последней проверке чётности Unraid более 40 дней: ежемесячная проверка пропущена (`dumbmonit_unraid_parity_check_age_seconds`). | > 40 дн | 1 ч | Advisory (`warning`) | 1 дн |
| Unraid cache nearly full | Кэш-пул Unraid заполнен более чем на 90 %: запись откатывается на медленный массив или завершается ошибкой (`dumbmonit_unraid_disk_used_percent{role="cache"}`). | > 90 % | 30 мин | Advisory (`warning`) | 1 дн |
| Unraid container stopped | Контейнер Docker, настроенный на запуск вместе с массивом, не запущен (`dumbmonit_unraid_container_autostart_stopped`). | > 0 | 10 мин | Advisory (`warning`) | 6 ч |

### Veeam Backup & Replication

См. [Veeam Backup & Replication](../devices/veeam.md). Задания помечены `job` и `type`, репозитории — `repository`.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Veeam job failed | Последний запуск включённого задания Veeam завершился неудачей: у защищаемых им данных нет новой точки восстановления (`dumbmonit_veeam_job_failed`). | > 0 | 5 мин | Warning (`critical`) | 1 дн |
| Veeam job ended with a warning | Последний запуск включённого задания Veeam завершился с предупреждением: некоторые объекты могут быть не защищены (`dumbmonit_veeam_job_warning`). | > 0 | 5 мин | Advisory (`warning`) | 1 дн |
| Veeam repository nearly full | Репозиторий резервных копий Veeam заполнен более чем на 90 %: следующие задания завершатся неудачей из-за нехватки места (`dumbmonit_veeam_repository_used_percent`). | > 90 % | 30 мин | Advisory (`warning`) | 1 дн |
| Veeam license expiring | Лицензия Veeam истекает менее чем через 30 дней: задания остановятся по окончании льготного периода (`dumbmonit_veeam_license_expiry_seconds`). | < 30 дн | 1 ч | Advisory (`warning`) | 1 дн |

### Tailscale

См. [Tailscale](../devices/tailscale.md). Устройства помечены `device` — первой меткой их имени MagicDNS.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Tailscale device offline | Устройство Tailscale, которое должно оставаться онлайн (по умолчанию устройство с тегом), отключено более 15 минут (`dumbmonit_tailscale_device_offline_seconds and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 15 мин | 5 мин | Advisory (`warning`) | 6 ч |
| Tailscale node key expiring | Ключ узла устройства Tailscale истекает менее чем через 14 дней: оно покинет tailnet, пока кто-нибудь не войдёт на нём снова (`dumbmonit_tailscale_device_key_expiry_seconds`). | < 14 дн | 1 ч | Advisory (`warning`) | 1 дн |
| Tailscale device awaiting approval | Устройство присоединилось к tailnet и ожидает одобрения: одобрите его, если оно ожидаемо, иначе удалите (`dumbmonit_tailscale_device_authorized`). | < 1 | 5 мин | Advisory (`warning`) | 1 дн |
| Tailscale client update | Устройство Tailscale, которое должно оставаться онлайн, работает с клиентом, для которого доступно обновление (`dumbmonit_tailscale_device_update_available and on (target, device) dumbmonit_tailscale_device_watched > 0`). | > 0 | 6 ч | Info (`info`) | 7 дн |

### FortiGate

См. [FortiGate](../devices/fortigate.md). Туннели помечены `tunnel`, интерфейсы — `interface` и `alias`, лицензии — `license`.

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| FortiGate IPsec tunnel down | Ни одна фаза 2 туннеля IPsec «сайт-сайт» FortiGate не поднята: сайт за ним отрезан (`dumbmonit_fortigate_ipsec_tunnel_up`). | < 1 | 5 мин | Warning (`critical`) | 6 ч |
| FortiGate interface without link | У включённого интерфейса FortiGate с адресом нет линка: кабель, трансивер или устройство напротив (`dumbmonit_fortigate_interface_down`). | > 0 | 5 мин | Advisory (`warning`) | 6 ч |
| FortiGate HA out of sync | У участников кластера HA FortiGate больше не совпадает контрольная сумма конфигурации: при переключении запустится другая конфигурация (`dumbmonit_fortigate_ha_in_sync`). | < 1 | 15 мин | Advisory (`warning`) | 6 ч |
| FortiGate HA member lost | В кластере HA FortiGate меньше участников, чем за последние 24 часа: резервного больше нет (`max_over_time(dumbmonit_fortigate_ha_members[1d]) - dumbmonit_fortigate_ha_members`). | > 0 | 5 мин | Warning (`critical`) | 6 ч |
| FortiGate memory near conserve mode | Память FortiGate остаётся выше 85 % десять минут: при 88 % (по умолчанию) он входит в conserve mode и перестаёт проверять новые сессии (`dumbmonit_fortigate_memory_used_percent`). | > 85 % | 10 мин | Advisory (`warning`) | 6 ч |
| FortiGate CPU high | CPU FortiGate остаётся выше 90 % пятнадцать минут (`dumbmonit_fortigate_cpu_usage_percent`). | > 90 % | 15 мин | Advisory (`warning`) | 6 ч |
| FortiGate licence expiring | Лицензия FortiGuard или FortiCare на FortiGate истекает менее чем через 30 дней или уже истекла (`dumbmonit_fortigate_license_expiry_seconds`). | < 30 дн | 1 ч | Advisory (`warning`) | 1 дн |
| FortiGate firmware update | FortiGuard предлагает более новый патч-выпуск ветки FortiOS, на которой работает FortiGate (`dumbmonit_fortigate_firmware_update_available`). | > 0 | 1 ч | Info (`info`) | 7 дн |

### Sophos Firewall

См. [Sophos Firewall](../devices/sophos.md).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Sophos interface without link | У включённого интерфейса Sophos Firewall, привязанного к зоне, нет линка (`dumbmonit_sophos_interface_down`). | > 0 | 5 мин | Advisory (`warning`) | 6 ч |

### KVM (libvirt)

Сообщается агентом Linux на хосте KVM; см. [Агент](../devices/agent.md#kvm-virtual-machines-through-libvirt).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| KVM domain crashed | libvirt сообщает о домене KVM как об аварийно завершившемся: гипервизору пришлось его остановить (`dumbmonit_agent_libvirt_domain_crashed`). | > 0 | 1 мин | Warning (`critical`) | 1 ч |
| libvirt pool almost full | Выделенное место пула хранилища libvirt достигает 90 % его ёмкости (`dumbmonit_agent_libvirt_pool_used_percent`). | ≥ 90 % (сбрасывается ниже 88 %) | 15 мин | Advisory (`warning`), эскалируется через 24 ч | 6 ч |

### XCP-ng / XenServer

См. [XCP-ng / XenServer](../devices/xcpng.md).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| XCP-ng host offline | Хост пула XCP-ng больше не отвечает координатору (`dumbmonit_xcpng_host_live`). | < 1 | 5 мин | Warning (`critical`) | 1 ч |
| XCP-ng SR almost full | Репозиторий хранилища (SR) XCP-ng заполнен более чем на 90 % (`dumbmonit_xcpng_sr_used_percent`). | ≥ 90 % (сбрасывается ниже 88 %) | 15 мин | Advisory (`warning`), эскалируется через 24 ч | 6 ч |
| XCP-ng HA-protected VM down | ВМ, защищённая высокой доступностью пула, остановлена (`dumbmonit_xcpng_vm_ha_protected_down`). | > 0 | 2 мин | Warning (`critical`) | 1 ч |
| XCP-ng pool HA overcommitted | У пула больше нет плана, как пережить отказы хостов, которые он настроен выдерживать (`dumbmonit_xcpng_pool_ha_overcommitted`). | > 0 | 5 мин | Advisory (`warning`), эскалируется через 1 ч | 6 ч |

### Nutanix

См. [Nutanix AHV](../devices/nutanix.md).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Nutanix host degraded | Prism сообщает о хосте Nutanix как о деградировавшем (`dumbmonit_nutanix_host_degraded`). | > 0 | 5 мин | Warning (`critical`) | 1 ч |
| Nutanix storage container almost full | Контейнер хранилища Nutanix заполнен более чем на 90 % (`dumbmonit_nutanix_storage_container_used_percent`). | ≥ 90 % (сбрасывается ниже 88 %) | 15 мин | Advisory (`warning`), эскалируется через 24 ч | 6 ч |
| Nutanix critical alerts | Prism Central сообщает об одном или нескольких неустранённых критических оповещениях (`dumbmonit_nutanix_alerts_critical`). | > 0 | 5 мин | Warning (`critical`) | 1 ч |

### Hyper-V

Сообщается агентом Windows на хосте Hyper-V; см. [Агент](../devices/agent.md#hyper-v-hosts).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Hyper-V VM health critical | Hyper-V сообщает о виртуальной машине в критическом состоянии: исчерпан необходимый ей ресурс, чаще всего место на диске (`dumbmonit_hyperv_vms_health_critical`). | > 0 | 5 мин | Warning (`critical`) | 6 ч |
| Hyper-V host CPU high | Логические процессоры хоста Hyper-V заняты более чем на 90 % пятнадцать минут, включая гостей (`dumbmonit_hyperv_host_cpu_percent`). | > 90 % | 15 мин | Advisory (`warning`) | 6 ч |
| Hyper-V virtual disk errors | Виртуальный диск Hyper-V сообщал об ошибках ввода-вывода за последний час (`increase_prometheus(dumbmonit_hyperv_vhd_errors_total[1h])`). | > 0 | 5 мин | Advisory (`warning`) | 6 ч |

### Оценка безопасности {#security-score}

Вычисляется сервером каждые 15 минут для каждого устройства, чей тип имеет проверки безопасности, и сохраняется как `dumbmonit_security_score` (0–100). См. [Оценка безопасности](../using/security-score.md).

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| Security score dropped | Оценка упала на 10 пунктов или более относительно лучшего значения за последние 24 часа (`max_over_time(dumbmonit_security_score[1d]) - dumbmonit_security_score`): какая-то проверка начала не проходить. | ≥ 10 пунктов | 30 мин | Advisory (`warning`) | 24 ч |
| Security score low | Оценка ниже 40, класс F (`dumbmonit_security_score`). Повышайте или понижайте порог в соответствии с вашей собственной планкой. | < 40 | 1 ч | Info (`info`) | 7 дн |

### Сам DumbMonit

| Правило | Что | Порог по умолчанию | Выдержка | Важность | Напоминание |
|---|---|---|---|---|---|
| DumbMonit backup did not run | Плановое локальное резервное копирование базы данных DumbMonit не запускалось более двух дней (`time() - dumbmonit_instance_backup_last_success_seconds`). | > 2 дн | 1 ч | Advisory (`warning`) | 24 ч |

Каждое встроенное правило применяется ко всем устройствам и ко всем включённым каналам.

## Редактирование правила в интерфейсе

На странице **Alerts** раздел *Rules* перечисляет каждое правило с его
важностью, пометкой *Built-in* и, когда уместно, числом оповещений, которые оно
сейчас порождает. У каждого правила есть:

- переключатель, чтобы **включить или отключить** его;
- **Edit**, раскрывающий встроенный редактор имени, описания, оператора,
  порога, выдержки (`for`), важности, напоминания (`repeat every`) и эскалации
  (`escalate after`). Запрос встроенного правила показывается, но не
  редактируется; запрос ваших собственных правил редактируется. Правила
  базовой линии показывают свои параметры обнаружения только для чтения;
- **Delete**, только для ваших собственных правил.

## Переопределения для устройства {#per-device-overrides}

Правило можно настроить для одного устройства, не трогая само правило: другой
порог, другой порог сброса или отключение только для этого устройства. В
редакторе каждого правила есть панель **Per-device overrides**, чтобы добавить
или убрать переопределение прямо там.

Действие **Ignore** в один клик над оповещением (в списке *Needs you* или на
странице устройства оповещения) — тот же механизм, записанный с `enabled: false`:
устройство немедленно и обратимо перестаёт нести это правило. Страница
устройства перечисляет игнорируемое с кнопкой **Stop ignoring**; само правило
показывает *Ignored on N devices* в Alerts → Rules, где та же панель
переопределений для устройства убирает его. API см. в
[Переопределениях для устройства](../reference/api.md#per-device-overrides).

## Создание порогового правила

Нажмите **New rule**. Форма требует:

| Поле | Значение |
|---|---|
| Name | Показывается в оповещениях и уведомлениях. |
| Query | Выражение MetricsQL, вычисляемое для каждого устройства. Пример: `dumbmonit_cpu_usage_percent`. |
| Operator | `>`, `>=`, `<` или `<=`. |
| Threshold | Число, с которым сравнивается результат запроса. |
| Severity | Info, Advisory или Warning. |
| Hold for | Секунды, в течение которых условие должно сохраняться до срабатывания. Ненулевая выдержка поглощает единичный плохой сэмпл. |

Каждый ряд, возвращённый запросом, становится одним потенциальным оповещением,
привязанным к устройству, названному его меткой `target`. Агрегируйте с помощью
`avg by (target, host) (…)`, когда нужно одно оповещение на устройство, а не на
ядро или интерфейс. Идентичность оповещения игнорирует метки `host` и `tag_*`,
если присутствует `target`, поэтому переименование устройства сохраняет его
оповещения; ряд, чей `target` не соответствует существующему включённому
устройству, игнорируется.

## Язык запросов

Запросы пишутся на [MetricsQL](https://docs.victoriametrics.com/metricsql/) —
надмножестве PromQL от VictoriaMetrics. Метрики — это ряды `dumbmonit_*`,
описанные в [справочнике метрик](../reference/metrics.md), и каждый ряд несёт
метку `target` (id устройства) и метку `host` (его имя).

Некоторые шаблоны, используемые встроенными правилами:

```
# Age of the last sample, freshest series per device: what "unreachable" means
bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d]))

# One value per device, whatever the source
avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)

# A percentage computed from two gauges
100 * dumbmonit_storage_bytes_used / dumbmonit_storage_bytes_total

# A boolean condition: the series is returned only when true
dumbmonit_ups_output_source == 5

# Extrapolation done by VictoriaMetrics
predict_linear(dumbmonit_pbs_datastore_used_percent[6h], 345600) and deriv(dumbmonit_pbs_datastore_used_percent[6h]) > 0

# Availability of a service over thirty days
avg_over_time(dumbmonit_probe_success[30d])
```

!!! tip "Сначала попробуйте запрос"
    `GET /api/metrics/query?query=…` возвращает то, что правило вычислило бы
    (см. [справочник API](../reference/api.md#metrics)). В разработке оверлей
    публикует встроенную VictoriaMetrics на `:8428` хоста вместе с её
    интерфейсом запросов.

## Виды правил в API

`kind` — это `threshold` (сравнение с порогом), `predict` (то же сравнение для
запроса `predict_linear`; показывается как прогноз) или `anomaly` (сезонная
базовая линия; `params` содержит `k`, `alpha`, `mad_floor_abs`, `mad_floor_rel`,
`min_samples`). Интерфейс создаёт правила `threshold`; два других вида можно
создавать через API.
