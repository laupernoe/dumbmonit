# Pacotes de integração {#integration-packs}

Um pacote de integração adiciona um tipo de monitor ao DumbMonit sem uma
nova versão. É um único arquivo YAML que diz onde ler (uma API HTTP ou uma
página Prometheus `/metrics` no monitor), o que extrair, e quais regras
de alerta vêm com ele. Uma vez instalado, o tipo do pacote aparece em
**Adicionar monitor** ao lado dos integrados, e seus monitores são
sondados, exibidos em gráfico e alertados como qualquer outro.

Um pacote é declarativo: sem código, sem scripts. O DumbMonit executa toda
requisição ele mesmo, sob regras que o pacote não pode alterar.

## O que um pacote pode e não pode fazer {#what-a-pack-can-and-cannot-do}

| Um pacote pode | Um pacote não pode |
|---|---|
| Chamar caminhos `GET` ou `POST` no próprio endereço do monitor | Chamar qualquer outro host, seguir um redirecionamento para outro host, ou usar uma URL absoluta |
| Ler JSON (JSONPath), texto simples (expressões regulares) ou o formato de texto do Prometheus | Executar código, comandos de shell ou templates além de `{{option.x}}` e `{{credential.x}}` |
| Enviar a credencial do monitor em um cabeçalho ou como autenticação básica/bearer | Colocar uma credencial em uma URL ou corpo de requisição |
| Declarar opções mostradas no formulário do monitor (um canal de chave, um limite) | Escrever métricas fora de seu próprio prefixo `dumbmonit_<pack>_` |
| Trazer regras de alerta em suas próprias métricas | Alterar um tipo de monitor integrado ou regra integrada |
| Trazer perfis SNMP, no formato dos integrados | Ler mais de 4 MiB por resposta, fazer mais de 16 requisições por sondagem, ou produzir mais de 5.000 séries por sondagem |

Endereços de loopback e link-local (`127.0.0.1`, `169.254.x.x`) são
recusados a menos que a opção **Permitir alvos de loopback e link-local**
do monitor esteja ativada, exatamente como nas verificações HTTP.
Endereços de LAN privados são sempre permitidos.

## Pacotes de referência {#reference-packs}

O repositório entrega três pacotes sob
[`packs/`](https://github.com/laupernoe/dumbmonit/tree/main/packs), cada um com
suas fixtures e saída esperada:

| Pacote | Lê | Regras |
|---|---|---|
| `node-exporter` | um subconjunto curado do Prometheus node_exporter (CPU, memória, sistemas de arquivos, discos, rede) | sistema de arquivos quase cheio, memória quase esgotada |
| `shelly-plug` | tomadas e relés Shelly Gen2+, `/rpc/Switch.GetStatus` | sobreaquecimento |
| `speedtest-tracker` | o resultado mais recente da API v1 do Speedtest Tracker | internet abaixo do esperado, teste de velocidade falhando |

## Instalar um pacote {#install-a-pack}

Na interface, como administrador: **Configurações → Pacotes de
integração**. Cole o YAML do pacote ou envie seu `pack.yaml`, depois
**Instalar o pacote**. Um pacote que não passa as verificações é recusado
com a lista do que corrigir; um que passa é salvo com suas notas, se houver.
A mesma página ativa e desativa pacotes e os desinstala. Tipos instalados
aparecem sob **Pacotes da comunidade** ao adicionar um monitor, marcados
*Pacote*.

Pela API HTTP, você precisa de uma sessão de administrador ou um token de
API com o escopo `write` (Configurações → **API e assistentes**).

```bash
# Instalar, ou atualizar para uma versão mais nova do mesmo pacote
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
| `GET /api/packs` | pacotes instalados: versão, tipo, métricas, regras, monitores que o usam |
| `GET /api/packs/{id}` | um pacote |
| `POST /api/packs` | instalar ou atualizar; corpo: o YAML (`application/yaml`), ou `{"yaml": "…"}` em JSON |
| `PUT /api/packs/{id}/disable` | parar de sondar os monitores do pacote, manter tudo mais |
| `PUT /api/packs/{id}/enable` | sondá-los de novo |
| `DELETE /api/packs/{id}` | desinstalar, com suas regras; recusado (`409`) enquanto monitores ainda o usam |

Com um cookie de sessão em vez de um token, adicione
`X-Requested-With: DumbMonit` a todo `POST`, `PUT` e `DELETE`, como para o
resto da [API](../reference/api.md).

Instalar tem efeito imediato: o novo tipo fica disponível no formulário de
monitor sem reinício. A única exceção é um pacote que traz perfis SNMP;
a resposta então diz `"restart_required": true`, porque o catálogo SNMP é
lido na inicialização.

### Atualizações e suas alterações {#updates-and-your-changes}

Atualizar um pacote (mesmo `id`, conteúdo novo) substitui sua extração e sua
descrição. Suas regras de alerta são apenas sempre *adicionadas*: uma regra
que você alterou, desativou ou excluiu nunca é reescrita por uma
atualização.

Desativar um pacote retira seu tipo de monitor e mantém tudo mais:
monitores, histórico, regras. Seus monitores não são mais sondados
(cada um mostra um erro de configuração) e, como qualquer monitor que
para de reportar, geram *Monitor unreachable* depois de três minutos: pause-os
também se você desativar o pacote por um tempo.

### Backups {#backups}

Pacotes instalados viajam em [backups](../install/backup.md) com seu YAML, e
são restaurados antes dos monitores que os usam.

## Verificar um pacote antes de instalá-lo {#check-a-pack-before-installing-it}

O binário do servidor valida e testa pacotes offline:

```bash
# Verificações de esquema e consistência, sem servidor ou banco de dados
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack lint /packs/shelly-plug

# Reproduz fixtures/ através da extração real e compara com expected.prom
docker run --rm -v "$PWD/packs:/packs:ro" ghcr.io/laupernoe/dumbmonit:latest \
  pack test /packs/shelly-plug
```

`pack test --update` reescreve `expected.prom` a partir das fixtures (monte
o diretório em leitura-escrita e execute com `--user "$(id -u)"` para que o
arquivo permaneça seu).

Próximo: [escreva seu primeiro pacote](first-pack.md) em cerca de dez
minutos, ou leia a [referência de formato](format.md).
