# Arquitetura

O DumbMonit é distribuído num único contentor. No interior há um processo Rust
que serve a interface web e a API, mais um processo filho (VictoriaMetrics) que
o processo Rust inicia e supervisiona. A configuração e o estado ficam no
SQLite.

## Componentes

| Componente | Onde | Função |
|---|---|---|
| Servidor | `crates/server` | O binário Rust único: API HTTP (axum), autenticação, agendador, motor de alertas, canais de notificação, relatórios. |
| Interface web | `web/` | Aplicação de página única SvelteKit (Svelte 5, Tailwind 4, uPlot). Compilada para ficheiros estáticos e incorporada no binário do servidor com `rust_embed`. |
| Séries temporais | `crates/server/src/tsdb/` | Escritor em lotes e proxy de consultas para o VictoriaMetrics. `embedded.rs` inicia e supervisiona o processo filho VictoriaMetrics incluído (dados em `/data/vm`); `DUMBMONIT_VM_URL` aponta antes para uma instância externa. |
| Configuração e estado | `crates/server/src/db/` | SQLite com migrações numeradas: dispositivos, credenciais, utilizadores, regras, estado dos alertas, silenciamentos, canais. |
| Coletores | `crates/collectors`, `crates/server/src/collectors/` | Uma implementação do trait `Collector` por tipo de dispositivo ou serviço, registada num `Registry`. O agendador só conhece o trait. |
| Tipos partilhados | `crates/proto` | `Sample`, `Target`, `Credential`, o trait `Collector` e `ProbeError`, e o protocolo dos agentes. |
| Agente | `crates/agent` | Agente para Linux, macOS, FreeBSD e Windows que envia métricas do anfitrião; pode também funcionar como relay, executando coletores para um local remoto. |
| Alertas | `crates/server/src/alerting/` | Regras, máquina de estados dos alertas, supressão pelo dispositivo pai, silenciamentos, linhas de base aprendidas. |
| Notificações | `crates/server/src/notify/` | Canais (chat, push, on-call, e-mail, webhooks), descritos à interface por `catalog.rs`. |
| Segredos | `crates/server/src/crypto.rs` | AES-256-GCM para as credenciais e os tokens guardados no SQLite; a chave vem de `/data/secret.key` ou de `DUMBMONIT_SECRET`. |

## Fluxo de dados

```text
 devices / services ──(SNMP, HTTP APIs, probes)──► collectors ┐
                                                              │ samples
 agents ──(push, per-machine token)──► /api/ingest ───────────┤
 relay agent (collectors run at a remote site) ───────────────┤
                                                              ▼
                                           tsdb writer (batched) ──► VictoriaMetrics
                                                                           │
 browser / scripts / assistants ◄─ /api/* ◄─ query proxy ◄─────────────────┘
                                   │
                                   └─► alerting engine ──► notify channels

 SQLite (config, users, alert state, silences) is read and written by the
 API, the scheduler and the alerting engine.
```

1. O agendador executa cada dispositivo ativo no respetivo intervalo através do
   registo de coletores. Os agentes e os relays enviam as suas medições para
   `/api/ingest` com um token por máquina.
2. Os coletores devolvem amostras chamadas `dumbmonit_<kind>_*` com uma etiqueta
   `target` (o id do dispositivo). O escritor agrupa-as em lotes para o
   VictoriaMetrics.
3. O motor de alertas avalia as regras sobre as séries armazenadas. `phase` é o
   estado do motor; `effective_phase` é o que os utilizadores veem
   (`suppressed` quando um dispositivo pai está em baixo). Um dispositivo
   inacessível gera um alerta; um erro de configuração é mostrado, mas não
   notificado.
4. As notificações saem pelos canais configurados na política de notificação.
5. A interface e qualquer script leem tudo através de `/api/*`; não existe uma
   metade privada da API.

## Pontos de extensão

- Um novo tipo de dispositivo é uma implementação de `Collector` mais uma
  chamada a `Registry::register`. Os formulários da interface são gerados a
  partir de `GET /api/collectors`.
- Um novo canal de notificação é um emissor mais uma entrada em
  `notify/catalog.rs`; a interface desenha o formulário a partir de
  `GET /api/notify/kinds`.
- Um dispositivo SNMP que só precisa de um perfil não precisa de Rust: um
  ficheiro YAML em `profiles/`.

Consulte [Desenvolvimento](../development.md) para as instruções de compilação
e [Modelo de segurança](security-model.md) para as fronteiras de confiança.
