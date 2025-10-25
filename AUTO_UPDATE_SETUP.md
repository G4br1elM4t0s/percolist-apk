# Sistema de Auto-Update do Percolist

Este documento explica como configurar e usar o sistema de atualização automática do Percolist.

## ✅ Status Atual

O sistema de auto-update está **completamente implementado** e funcionando:

- ✅ Plugin de atualização configurado
- ✅ Plugin de dialog configurado
- ✅ Chaves de assinatura geradas
- ✅ Permissões configuradas
- ✅ Código de verificação implementado
- ✅ Comando de restart implementado

## 🔧 Configuração Atual

### Chaves de Assinatura
- **Chave Privada**: `~/.tauri/percolist.key`
- **Chave Pública**: `dW50cnVzdGVkIGNvbW1lbnQ6IEU5NkEwMjYxNENERDBDNjIKUldSaUROMU1ZUUpxNmVmWEp0d0QzNDFUbFlrZ1VablNCdml2Z1NxME9qTzhvTm1KaTVZOVZ5OTYK`

### Endpoint de Atualizações
- **URL**: `https://raw.githubusercontent.com/seu-usuario/seu-repo/main/latest.json`
- **Status**: ⚠️ **PRECISA SER CONFIGURADO**

## 📋 Próximos Passos

### 1. Configurar o Repositório GitHub

1. **Crie um repositório público** no GitHub (se o projeto for privado)
2. **Atualize a URL** no `src-tauri/tauri.conf.json`:
   ```json
   "endpoints": [
     "https://raw.githubusercontent.com/SEU_USUARIO/SEU_REPO/main/latest.json"
   ]
   ```

### 2. Configurar o latest.json

1. **Faça upload** do arquivo `latest.json` para o seu repositório
2. **Atualize as URLs** no arquivo para apontar para os releases corretos

### 3. Configurar Variáveis de Ambiente

Antes de fazer build, configure as variáveis de ambiente:

```bash
export TAURI_SIGNING_PRIVATE_KEY="~/.tauri/percolist.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="SUA_SENHA_AQUI"
```

### 4. Processo de Release

#### Opção A: Manual
1. Atualize as versões nos arquivos:
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
   - `package.json`

2. Faça o build:
   ```bash
   npm run tauri build
   ```

3. Gere as assinaturas:
   ```bash
   npm run tauri signer sign src-tauri/target/release/bundle/nsis/Percolist_Panel_1.0.2_x64-setup.exe
   ```

4. Crie um release no GitHub com os arquivos

5. Atualize o `latest.json` com as novas informações

#### Opção B: Automatizado
Use o script fornecido:
```bash
./build-and-release.sh 1.0.2 "Correções de bugs e melhorias"
```

## 🔄 Como Funciona

### Para o Usuário
1. **Inicialização**: O app verifica automaticamente atualizações
2. **Atualização Disponível**: Dialog nativo aparece com opções
3. **Download**: App baixa e instala automaticamente
4. **Restart**: App reinicia automaticamente

### Para o Desenvolvedor
1. **Build**: Gera binários assinados
2. **Release**: Upload para GitHub
3. **latest.json**: Atualiza com novas informações
4. **Usuários**: Recebem atualização automaticamente

## 🛠️ Comandos Úteis

### Gerar Chaves (já feito)
```bash
npm run tauri signer generate -- -w ~/.tauri/percolist.key
```

### Build com Assinatura
```bash
export TAURI_SIGNING_PRIVATE_KEY="~/.tauri/percolist.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="sua_senha"
npm run tauri build
```

### Assinar Arquivo
```bash
npm run tauri signer sign arquivo.exe
```

### Testar Atualizações
```bash
npm run tauri dev
```

## 📁 Arquivos Importantes

- `src-tauri/tauri.conf.json` - Configuração do updater
- `src/App.tsx` - Código de verificação de atualizações
- `latest.json` - Metadados das atualizações
- `build-and-release.sh` - Script de automação

## 🔍 Troubleshooting

### Erro: "Falha ao verificar atualizações"
- Verifique se a URL do endpoint está correta
- Confirme se o `latest.json` está acessível
- Verifique se as chaves estão configuradas

### Erro: "Assinatura inválida"
- Verifique se a chave pública está correta no `tauri.conf.json`
- Confirme se o arquivo foi assinado corretamente
- Verifique se a senha da chave privada está correta

### App não reinicia após atualização
- Verifique se o comando `graceful_restart` está funcionando
- Confirme se as permissões estão configuradas

## 🎯 Próximas Melhorias

1. **GitHub Actions**: Automatizar build e release
2. **Notificações**: Sistema de notificações para atualizações
3. **Rollback**: Sistema de reversão de versões
4. **Beta Channel**: Canal de atualizações beta
5. **Progress Bar**: Mostrar progresso do download

## 📞 Suporte

Para dúvidas ou problemas:
1. Verifique os logs do app
2. Confirme as configurações
3. Teste com uma versão de exemplo
4. Consulte a documentação do Tauri v2
