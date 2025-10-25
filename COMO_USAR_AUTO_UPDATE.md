# 🚀 Como Usar o Auto-Update - Guia Rápido

## ⚡ Configuração Rápida (5 minutos)

### 1. Execute o script de configuração:
```bash
./setup-update.sh
```

### 2. Siga as instruções na tela:
- Digite seu usuário do GitHub
- Digite o nome do repositório
- Digite a senha da chave privada

### 3. Faça upload do latest.json:
- Vá para seu repositório no GitHub
- Faça upload do arquivo `latest.json` na raiz

### 4. Configure as variáveis de ambiente:
```bash
source .env.update
```

### 5. Faça o build:
```bash
npm run tauri build
```

### 6. Crie um release no GitHub:
- Vá para "Releases" no seu repositório
- Crie um novo release com os arquivos de `src-tauri/target/release/bundle/`

### 7. Teste:
```bash
npm run tauri dev
```

## 🔄 Como Fazer Releases

### Opção A: Automatizado
```bash
./build-and-release.sh 1.0.2 "Correções de bugs"
```

### Opção B: Manual
1. Atualize as versões nos arquivos
2. `npm run tauri build`
3. Crie release no GitHub
4. Atualize o `latest.json`

## 🎯 Como Funciona

1. **App inicia** → Verifica atualizações automaticamente
2. **Atualização encontrada** → Dialog nativo aparece
3. **Usuário confirma** → Download e instalação automática
4. **App reinicia** → Nova versão ativa

## 🔧 Troubleshooting

### Erro: "Falha ao verificar atualizações"
- Verifique se o `latest.json` está no GitHub
- Confirme se a URL está correta no `tauri.conf.json`

### Erro: "Assinatura inválida"
- Verifique se a senha da chave está correta
- Confirme se o arquivo foi assinado

### App não reinicia
- Verifique se as permissões estão configuradas
- Confirme se o comando `graceful_restart` funciona

## 📞 Suporte

Se algo não funcionar:
1. Verifique os logs do app
2. Confirme as configurações
3. Teste com uma versão de exemplo
4. Consulte `AUTO_UPDATE_SETUP.md` para detalhes completos
