# Cópia de segurança e restauro {#backup-and-restore}

Duas coisas diferentes, para dois acidentes diferentes.

| | Pacote encriptado | Cópia de segurança local agendada |
|---|---|---|
| O que contém | A configuração: dispositivos e as suas credenciais, regras, canais, páginas de estado, contas, tokens. | A base de dados inteira, como um ficheiro, mais `secret.key`. |
| Onde vai | Onde quiser — é encriptado com uma frase-passe que escolhe. | `/data/backups/`, dentro do mesmo volume. |
| Do que o salva | Um servidor perdido, uma mudança para hardware novo, uma instalação nova. | A alteração de que se arrepende dez minutos depois, uma atualização que correu mal. |
| O que não contém | Histórico de métricas, estado e histórico de alertas, o registo de auditoria, sessões. | Histórico de métricas (`vm/`). |

Nenhum substitui um arquivo de todo o volume se também quiser os
gráficos de volta; isso está no fundo desta página.

!!! danger "`/data/secret.key` é o que desencripta as credenciais dos seus dispositivos"
    Comunidades SNMP, tokens Proxmox, senhas SMTP e URLs de heartbeat
    são guardados encriptados com uma chave derivada do segredo da
    instância. No primeiro arranque o DumbMonit escreve-o em
    `/data/secret.key` (ou lê-o de `DUMBMONIT_SECRET` se definir um).

    **Uma cópia da base de dados sem esse ficheiro restaura uma
    instância que não consegue falar com nada.** O servidor nota isso
    e recusa arrancar com uma mensagem explícita em vez de falhar em
    cada verificação — mas as credenciais desapareceram, e todo
    dispositivo tem de ser introduzido de novo.

    O pacote encriptado é a única excepção: os seus segredos são
    reencriptados com a sua frase-passe, por isso restaura numa
    instância nova que tem o seu próprio segredo, novinho em folha.

## O pacote encriptado {#the-encrypted-bundle}

**Settings → Backup → Download the bundle.** Escolha uma frase-passe
de pelo menos 16 caracteres — três palavras bastam — e confirme-a. O
ficheiro que desce é JSON simples com um corpo encriptado:

```json
{
  "format": "dumbmonit-backup",
  "version": 1,
  "created_at": "2026-09-22 19:04:11",
  "source_version": "0.1.0-alpha.3",
  "summary": { "targets": 12, "channels": 2, "rules": 41 },
  "kdf": { "algorithm": "argon2id", "salt": "…", "m_cost": 32768, "t_cost": 3, "p_cost": 1 },
  "cipher": "aes-256-gcm",
  "payload": "…"
}
```

O cabeçalho fica legível para que saiba o que é um ficheiro antes de
escrever nada. Tudo o resto — endereços incluídos — está dentro de
`payload`, encriptado com AES-256-GCM sob uma chave derivada da sua
frase-passe pelo Argon2id. **Não há recuperação**: um pacote cuja
frase-passe se perdeu está perdido.

!!! warning "O pacote contém toda credencial desta instância"
    Quem tiver o ficheiro e a frase-passe pode ler a senha de todo
    dispositivo que monitoriza. Mantenha-o onde guarda senhas.

### O que contém {#what-is-in-it}

| Secção | Conteúdo |
|---|---|
| `targets` | Dispositivos: nome, tipo, endereço, intervalo, opções, pai, agente relay, e a credencial. |
| `rules` | Regras de alerta, incluindo as incorporadas que reajustou ou desativou. |
| `rule_overrides` | Limiares e exclusões por dispositivo. |
| `channels` | Canais de notificação, as suas definições, a sua política por canal e os seus segredos. |
| `notify_policy` | A política de notificação global (agrupamento, limite horário, flapping, URL público). |
| `silences` | Janelas de manutenção. |
| `status_pages` | Páginas de estado públicas e os serviços que cada uma lista. |
| `incidents` | Incidentes e anúncios de manutenção, com as suas atualizações. |
| `users` | Contas: nome de utilizador, nome de exibição, função, ligação SSO, ativada ou não. |
| `agent_tokens` | Tokens de registo de agente, como hashes. Os agentes instalados continuam a enviar depois de um restauro. |
| `api_tokens` | Tokens de API, como hashes. Os tokens existentes continuam a funcionar. |
| `push_monitors` | Tokens de heartbeat, para que os URLs que as suas tarefas cron já chamam permaneçam os mesmos. |

Os dispositivos são associados por **tipo + endereço**, que é também a
chave única da tabela: importar o mesmo pacote duas vezes nunca cria
uma segunda cópia de nada.

### Senhas de conta {#account-passwords}

Por predefinição as contas são exportadas **sem** o seu hash de senha,
segredo TOTP ou códigos de recuperação. As contas restauradas então
existem com o nome e função certos, mas ninguém pode iniciar sessão
com elas até um administrador definir uma senha — ou até iniciarem
sessão via SSO, o que não precisa de nada do pacote.

Ative **Include account passwords and 2FA secrets** quando está a
mudar uma instância em vez de copiar a sua configuração. Um hash
Argon2 num ficheiro que viaja pode ser atacado offline durante o tempo
que o atacante quiser; a predefinição é a segura.

### O que não contém {#what-is-not-in-it}

O histórico de métricas (que vive na VictoriaMetrics, não na base de
dados), estado e histórico de alertas, linhas de base de anomalias, o
registo de auditoria, sessões abertas, o registo e última presença de
agentes, e as caches de verificação dos painéis Proxmox e Synology.
São observações, não definições: voltam por si próprias.

E **não o segredo da instância**. Esse é o objetivo: o pacote
transporta os seus segredos reencriptados sob a sua frase-passe
precisamente para que possa ser restaurado em algum lugar que nunca
viu `secret.key`.

### Restaurar numa instância nova {#restoring-onto-a-fresh-instance}

1. Inicie o contentor novo e crie a primeira conta de administrador em
   `/setup`, com o código de configuração impresso nos seus registos.
   Essa conta é sua; o restauro não a tocará.
2. **Settings → Backup → Restore a bundle.** Escolha o ficheiro,
   escreva a frase-passe, e pressione **Check this backup**.
3. Leia o relatório. Diz, secção por secção, o que seria **criado**, o
   que seria **atualizado** porque difere do que já está aqui, e o que
   já é idêntico — mais uma nota para qualquer coisa que não pôde
   colocar, como uma regra que apontava para um dispositivo que o
   pacote não contém.
4. Pressione **Restore for real**.

Três regras que o restauro nunca quebra:

- **Nada é alguma vez eliminado.** Um restauro faz existir o que o
  pacote descreve; qualquer coisa que a instância tenha além disso
  permanece.
- **Nada é alguma vez duplicado.** Todo item é associado pela sua
  chave natural — tipo + endereço para um dispositivo, `uid` para uma
  regra, nome para um canal, slug para uma página de estado. Restaurar
  o mesmo pacote duas vezes reporta tudo como já idêntico e não
  escreve nada.
- **As contas existentes são deixadas em paz.** Uma conta cujo nome de
  utilizador já está aqui é ignorada, nunca sobrescrita: um ficheiro
  não deve poder alterar a função ou a senha do administrador que
  executa o restauro.

Um ensaio não é uma simulação do restauro, *é* o restauro, executado
dentro de uma transação que é revertida ao fim. O relatório não pode
estar errado sobre o que aplicar faria.

Um pacote escrito por um DumbMonit mais recente é recusado com uma
frase que o diz — atualize primeiro, depois restaure. Um pacote que
foi modificado, ou cuja frase-passe está errada, também é recusado:
o AES-GCM não consegue distinguir os dois, e nem a mensagem.

### A partir da linha de comandos {#from-the-command-line}

Ambos os caminhos precisam de uma **sessão** de administrador, não de
um token de API: um token vive num script, e um script não tem
motivo para exportar toda credencial da instância.

```bash
# Iniciar sessão
curl -s -c jar -X POST http://localhost:8080/api/auth/login \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"username":"admin","password":"…"}'

# Exportar
curl -s -b jar -X POST http://localhost:8080/api/backup \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"passphrase":"three words are enough"}' -o backup.json

# Ensaio, depois aplicar
jq -n --slurpfile b backup.json \
  '{bundle: $b[0], passphrase: "three words are enough", apply: false}' > restore.json
curl -s -b jar -X POST http://localhost:8080/api/backup/restore \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d @restore.json | jq
```

## Cópias de segurança locais agendadas {#scheduled-local-backups}

O DumbMonit escreve uma cópia da sua própria base de dados numa
agenda, por predefinição **uma vez por dia, mantendo as últimas
sete**, em `/data/backups/`:

```
/data/backups/dumbmonit-20260922-030000.db
/data/backups/dumbmonit-20260922-030000.key
```

O ficheiro `.db` é produzido com o `VACUUM INTO` do SQLite: uma cópia
em funcionamento, consistente, escrita enquanto o servidor continua a
correr, não uma cópia de ficheiro que poderia apanhar a base de dados
a meio de uma escrita. O `sqlite3` abre-o como está, sem nenhum
journal para repetir. O `.key` junto a ele é a cópia do `secret.key`
— juntos, o par é suficiente para reconstruir a instância.

Vivem no mesmo volume que a base de dados, por isso desfazem um erro,
não um disco perdido. Copie-os para fora da máquina se quiser mais do
que isso.

### Definições {#settings}

| Variável | Predefinição | Função |
|---|---|---|
| `DUMBMONIT_BACKUP_ENABLED` | `1` | `0`, `false`, `no` ou `off` para parar de escrever cópias de segurança agendadas. |
| `DUMBMONIT_BACKUP_DIR` | `<diretório de dados>/backups` | Onde são escritas. Aponte-o para um segundo volume para sobreviver à perda do primeiro. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Horas entre duas cópias de segurança. A primeira acontece um intervalo depois do arranque, não no arranque. |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Quantas são mantidas. As mais antigas são removidas, com o seu `.key`. |

Nada mais no diretório é alguma vez tocado: só ficheiros chamados
`dumbmonit-<timestamp>.db` e o seu `.key` são rotacionados.

Quando o segredo da instância vem de `DUMBMONIT_SECRET` não há nenhum
ficheiro para copiar, e nenhum é escrito — guarde esse valor no seu
gestor de senhas em vez disso. A secção Backup diz em qual dos dois
casos está.

### Saber que correu {#knowing-that-it-ran}

Cada execução publica três séries:

| Métrica | Significado |
|---|---|
| `dumbmonit_instance_backup_last_success_seconds` | Data-hora Unix da última cópia de segurança **bem-sucedida**. Uma tentativa falhada nunca a atualiza. |
| `dumbmonit_instance_backup_last_status` | `1` se a última tentativa teve sucesso, `0` se falhou. |
| `dumbmonit_instance_backup_size_bytes` | Tamanho do último ficheiro escrito. |

A regra incorporada **DumbMonit backup did not run**
(`instance_backup_missing`) dispara quando o último sucesso tem mais
de dois dias — uma noite perdida pode ser um reinício, duas é um
problema. A série só existe uma vez a primeira cópia de segurança
escrita, por isso uma instância com a agenda desligada nunca é
incomodada com isso.

**Settings → Backup** mostra a mesma coisa sem MetricsQL: a última
execução e se teve sucesso, os ficheiros mantidos e o seu tamanho, e
**Back up now** para escrever uma de imediato.

### Restaurar uma {#restoring-one}

Pare a stack, coloque o ficheiro de volta sob o nome da base de dados,
e restaure a chave junto a ele:

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data alpine sh -c '
  cp /data/backups/dumbmonit-20260922-030000.db /data/dumbmonit.db &&
  cp /data/backups/dumbmonit-20260922-030000.key /data/secret.key &&
  rm -f /data/dumbmonit.db-wal /data/dumbmonit.db-shm'
docker compose start
```

Remover os ficheiros `-wal` e `-shm` importa: pertencem à base de
dados que acabou de substituir, e o SQLite tentaria aplicá-los à nova.

Para ler uma cópia de segurança sem a restaurar, copie-a para fora e
abra-a com `sqlite3`:

```bash
docker run --rm -v dumbmonit-data:/data -v "$PWD:/out" alpine \
  cp /data/backups/dumbmonit-20260922-030000.db /out/
sqlite3 dumbmonit-20260922-030000.db 'SELECT name, kind, address FROM targets;'
```

As credenciais nesse ficheiro continuam ilegíveis sem o `secret.key`
correspondente.

## Arquivar todo o volume {#archiving-the-whole-volume}

A única forma de manter também os gráficos. Um volume nomeado,
`dumbmonit-data`, contém tudo: a base de dados, o segredo, as cópias
de segurança locais e os dados da VictoriaMetrics.

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Para restaurar, crie o volume, extraia o arquivo para dentro dele da
mesma forma, depois `docker compose up -d`.

Se o espaço importa, `--exclude=./vm` mantém o arquivo pequeno: a base
de dados e o segredo são a configuração, `vm/` é só o histórico.

## O que fazer antes de uma atualização {#what-to-do-before-an-upgrade}

1. **Settings → Backup → Back up now** — trinta segundos, e inclui o
   segredo.
2. Descarregue também um pacote se a atualização for grande, ou se
   estiver a mudar para outra máquina.
3. `docker compose pull && docker compose up -d`. As migrações correm
   no arranque.

Se correr mal, [restaure a cópia de segurança local](#restoring-one) e
inicie a etiqueta de imagem anterior.
