# Dispositivo de demonstração {#demo-device}

Um dispositivo fictício que produz medições, para explorar a ferramenta sem
hardware. Não é necessário qualquer hardware.

## Configuração {#setup}

1. Dê-lhe um nome qualquer e confirme.
2. As medições fictícias aparecem de imediato, o suficiente para explorar a interface.

O endereço é livre (o marcador de posição diz `demo`), não há credencial nem
opção. O dispositivo de demonstração produz uma série `cpu_usage_percent`, pelo
que as regras «High CPU» e «Unusual CPU» aplicam-se a ele, e pode ser usado como
pai ou filho numa [cadeia de dependências](../alerting/index.md#dependency-suppression)
para ver a supressão em ação.

Para medições reais sem hardware dedicado, aponte um dispositivo SNMP a
qualquer máquina com `snmpd` em execução: ver [Testar sem hardware](../install/docker.md#testing-without-hardware).
