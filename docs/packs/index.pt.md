# Pacotes de integração {#integration-packs}

Um pacote de integração adiciona um tipo de dispositivo ao DumbMonit sem uma
nova versão. É um único ficheiro YAML que diz onde ler (uma API HTTP ou uma
página Prometheus `/metrics` no dispositivo), o que extrair, e quais regras
de alerta vêm com ele. Uma vez instalado, o tipo do pacote aparece em **Add
device** ao lado dos incorporados, e os seus dispositivos são sondados,
representados em gráficos e alertados como qualquer outro.

Um pacote é declarativo: sem código, sem scripts. O DumbMonit executa cada
pedido ele próprio, sob regras que o pacote não pode alterar.

## O que um pacote pode e não pode fazer {#what-a-pack-can-and-cannot-do}

| Um pacote pode | Um pacote não pode |
|---|---|
| Chamar caminhos `GET` ou `POST` no próprio endereço do dispositivo | Chamar qualquer outro anfitrião, seguir um redirecionamento para outro anfitrião, ou usar um URL absoluto |
| Ler JSON (JSONPath), texto simples (expressões regulares) ou o formato de texto Prometheus | Executar código, comandos de shell ou modelos além de `{{option.x}}` e `{{credential.x}}` |
| Enviar a credencial do dispositivo num cabeçalho ou como autenticação básica/bearer | Colocar uma credencial num URL ou no corpo de um pedido |
| Declarar opções mostradas no formulário do dispositivo (um canal de interruptor, um limiar) | Escrever métricas fora do seu próprio prefixo `dumbmonit_<pack>_` |
| Trazer regras de alerta sobre as suas próprias métricas | Alterar um tipo de dispositivo ou regra incorporada |
| Trazer perfis SNMP, no formato dos incorporados | Ler mais de 4 MiB por resposta, fazer mais de 16 pedidos por sondagem, ou produzir mais de 5000 séries por sondagem |

Endereços de loopback e link-local (`127.0.0.1`, `169.254.x.x`) são
recusados a menos que a opção **Allow loopback and link-local targets** do
dispositivo esteja ativa, exatamente como as verificações HTTP. Endereços
de LAN privada são sempre permitidos.

## Pacotes de referência {#reference-packs}

O repositório traz três pacotes sob
[`packs/`](https://github.com/laupernoe/dumbmonit/tree/main/packs), cada um
com os seus fixtures e a saída esperada:

| Pacote | Lê | Regras |
|---|---|---|
| `node-exporter` | um subconjunto selecionado do Prometheus node_exporter (CPU, memória, sistemas de ficheiros, discos, rede) | sistema de ficheiros quase cheio, memória quase esgotada |
| `shelly-plug` | tomadas e relés Shelly Gen2+, `/rpc/Switch.GetStatus` | sobreaquecimento |
| `speedtest-tracker` | o resultado mais recente da API v1 do Speedtest Tracker | internet abaixo do esperado, teste de velocidade a falhar |

## Instalar um pacote {#install-a-pack}

Na interface, como administrador: **Settings → Integration packs**. Cole o
YAML do pacote ou envie o seu `pack.yaml`, depois **Install the pack**. Um
pacote que não passa as verificações é recusado com a lista do que
corrigir; um que passa é guardado com as suas notas, se existirem. A mesma
página ativa e desativa pacotes e desinstala-os. Os tipos instalados
aparecem em **Community packs** ao adicionar um dispositivo, marcados
*Pack*.

Através da API HTTP, precisa de uma sessão de administrador ou um token de
API com o âmbito `write` (Settings → **API & assistants**).

```bash
# Instalar, ou atualizar para uma versão mais recente do mesmo pacote
curl -H 'Authorization: Bearer dmt_…' -H 'Content-Type: application/yaml' \
  --data-binary @packs/shelly-plug/pack.yaml http://localhost:8080/api/packs

# Listar o que está instalado
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/api/packs
```

A resposta diz o que aconteceu (`created`, `updated` ou `unchanged`), as
métricas que o pacote produz e quantas regras de alerta foram adicionadas.
Um pacote que não passa a validação é recusado com a lista completa de
problemas, um por linha.

| Chamada | Faz |
|---|---|
| `GET /api/packs` | pacotes instalados: versão, tipo, métricas, regras, dispositivos a usá-lo |
| `GET /api/packs/{id}` | um pacote |
| `POST /api/packs` | instalar ou atualizar; corpo: o YAML (`application/yaml`), ou `{"yaml": "…"}` em JSON |
| `PUT /api/packs/{id}/disable` | parar de sondar os dispositivos do pacote, manter tudo o resto |
| `PUT /api/packs/{id}/enable` | sondá-los de novo |
| `DELETE /api/packs/{id}` | desinstalar, com as suas regras; recusado (`409`) enquanto dispositivos ainda o usam |

Com um cookie de sessão em vez de um token, adicione
`X-Requested-With: DumbMonit` a cada `POST`, `PUT` e `DELETE`, como para o
resto da [API](../reference/api.md).

Instalar tem efeito imediato: o novo tipo fica disponível no formulário de
dispositivo sem reiniciar. A única excepção é um pacote que traz perfis
SNMP; a resposta diz então `"restart_required": true`, porque o catálogo
SNMP é lido no arranque.

### Atualizações e as suas alterações {#updates-and-your-changes}

Atualizar um pacote (mesmo `id`, novo conteúdo) substitui a sua extração e
a sua descrição. As suas regras de alerta só são *adicionadas*: uma regra
que alterou, desativou ou eliminou nunca é reescrita por uma atualização.

Desativar um pacote retira o seu tipo de dispositivo e mantém tudo o resto:
dispositivos, histórico, regras. Os seus dispositivos deixam de ser
sondados (cada um mostra um erro de configuração) e, como qualquer
dispositivo que deixa de reportar, geram *Device unreachable* depois de
três minutos: pause-os também se desativar o pacote por algum tempo.

### Cópias de segurança {#backups}

Os pacotes instalados viajam nas [cópias de segurança](../install/backup.md)
com o seu YAML, e são restaurados antes dos dispositivos que os usam.

## Verificar um pacote antes de o instalar {#check-a-pack-before-installing-it}

O binário do servidor valida e testa pacotes offline:

```bash
# Verificações de esquema e consistência, sem servidor nem base de dados
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack lint /packs/shelly-plug

# Reproduz fixtures/ através da extração real e compara com expected.prom
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /packs/shelly-plug
```

`pack test --update` reescreve `expected.prom` a partir dos fixtures (monte
o diretório em leitura-escrita e execute com `--user "$(id -u)"` para que o
ficheiro continue seu).

A seguir: [escreva o seu primeiro pacote](first-pack.md) em cerca de dez
minutos, ou leia a [referência de formato](format.md).
