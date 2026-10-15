# Expiração de domínio {#domain-expiry}

Um domínio registrado, lido de seu registro via RDAP: os dias restantes antes
de expirar, uma retenção que o tira do DNS, um período de resgate, seu
registrador e se sua delegação está assinada com DNSSEC.

O RDAP é o sucessor do WHOIS: os mesmos dados de registro, em JSON, com um
formato padrão entre registros. O DumbMonit encontra o registro da extensão no
arquivo de bootstrap RDAP da IANA
(`https://data.iana.org/rdap/dns.json`, lido no máximo uma vez por dia), e
então consulta `/domain/example.com`. Nenhuma credencial é necessária, e nada
além do registro do domínio é solicitado.

Duas falhas valem a pena observar além da própria data de expiração. Um
domínio em `clientHold` ou `serverHold` foi tirado do DNS pelo registrador ou
pelo registro, geralmente por uma fatura não paga ou um endereço de contato
que nunca foi verificado: todo site e caixa de e-mail sob ele para
imediatamente. Um domínio em `redemptionPeriod` ou `pendingDelete` já expirou
e será liberado a menos que seja restaurado, por um preço.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_domain_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `expiry_days` | dias restantes antes da expiração, fracionário; negativo uma vez expirado | |
| `expiry_timestamp_seconds` | a data de expiração, em segundos Unix | |
| `expiry_published` | 0 quando o registro não publica a data de expiração | |
| `on_hold` | 1 em `clientHold` ou `serverHold` | |
| `redemption` | 1 em `redemptionPeriod`, `pendingDelete` ou `pendingRestore` | |
| `status` | valor 1, uma série por código de status, na forma EPP (`clientTransferProhibited`) | `status` |
| `registrar_info` | valor 1 | `registrar` |
| `dnssec` | 1 quando a delegação está assinada | |
| `rdap_ok` | 0 quando a última consulta ao registro falhou e uma resposta anterior é mostrada | |
| `rdap_age_seconds` | idade da resposta mostrada | |

O registro é consultado uma vez por intervalo de atualização (12 horas por
padrão); no intervalo, os dias restantes são recalculados a cada verificação.
Quando o registro não responde, a última resposta ainda é usada por até sete
dias, com `rdap_ok` em 0, então uma indisponibilidade do registro não parece
um problema do domínio.

As [regras integradas](../alerting/rules.md#reverse-proxies-and-domains) que
se aplicam: Domain expiring (menos de 30 dias), Domain about to expire (menos
de 7 dias ou expirado), Domain on hold, Domain in redemption, além de Device
unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o registro. Ele mostra a data de expiração e os dias
restantes, os códigos de status (ou a retenção e o resgate em uma frase), o
registrador e o DNSSEC, e se a última consulta ao registro falhou.

## Observar o registro de um domínio {#watch-a-domains-registration}

1. Informe o domínio registrado, por exemplo "example.com", não um nome sob
   ele como www.example.com, que o registro não conhece. Nada a configurar e
   nenhuma credencial: o RDAP, sucessor do WHOIS, é público.

2. O DumbMonit encontra o registro da extensão no arquivo de bootstrap RDAP
   da IANA, e então consulta a data de expiração, os códigos de status e o
   registrador, duas vezes ao dia por padrão. Registros limitam consultas
   RDAP, e a data de expiração muda uma vez por ano: um intervalo de
   atualização menor não ganha nada.

3. Alguns registros executam RDAP sem estarem listados pela IANA, os de .ch e
   .li por exemplo: defina Servidor RDAP para o endereço deles. Uma extensão
   cujo registro não executa um serviço RDAP público não pode ser observada.

    ```
    https://rdap.nic.ch/
    ```

4. Alguns registros não publicam a data de expiração (.ch é um deles): o
   DumbMonit então avisa isso, e ainda observa os códigos de status.

!!! warning
    Um domínio em clientHold ou serverHold é tirado do DNS: todo site e caixa
    de e-mail sob ele para, geralmente por uma fatura não paga ou um endereço
    de contato não verificado. O DumbMonit avisa 30 dias antes da expiração,
    eleva 7 dias antes, e alerta imediatamente em uma retenção ou um período
    de resgate.

Um domínio que o registro não conhece (um subdomínio, um erro de digitação,
ou um domínio que foi excluído) é reportado como um erro de configuração, não
como uma indisponibilidade. Nomes com acentos são aceitos e consultados em sua
forma ASCII (`xn--…`).

## Credenciais {#credentials}

Nenhuma: o RDAP é público.

Endereço: o domínio registrado (`example.com`); uma URL colada é reduzida ao
seu nome de host. Opções: Servidor RDAP (vazio: encontrado via IANA),
intervalo de atualização (12 horas, de 1 a 168) e tempo limite da requisição
(10 s). O intervalo de verificação do próprio monitor pode ficar no
padrão: ele apenas recalcula os dias restantes entre duas consultas ao
registro.
