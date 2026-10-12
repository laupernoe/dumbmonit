# Dispositivo de demonstração

Um dispositivo fictício produzindo medições, para explorar a ferramenta sem
hardware. Nenhum hardware necessário.

## Configuração

1. Dê a ele qualquer nome e confirme.
2. Medições falsas aparecerão imediatamente, suficientes para navegar pela
   interface.

O endereço é livre (o placeholder diz `demo`), não há credencial e nenhuma
opção. O dispositivo de demonstração produz uma série `cpu_usage_percent`,
então as regras "High CPU" e "Unusual CPU" se aplicam a ele, e pode ser
usado como pai ou filho em uma
[cadeia de dependência](../alerting/index.md#dependency-suppression) para
ver a supressão em ação.

Para medições reais sem hardware dedicado, aponte um dispositivo SNMP para
qualquer máquina rodando `snmpd`: veja
[Testando sem hardware](../install/docker.md#testing-without-hardware).
