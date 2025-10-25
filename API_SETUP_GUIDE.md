# 🚀 Configuração com API Percolist

## ✅ Status Atual

Sua configuração já está pronta para usar `https://api.percolist.com.br/updates/{{target}}/{{arch}}/latest.json`!

## 📋 O que você precisa fazer na sua API

### 1. Implementar os Endpoints

Sua API precisa responder aos seguintes endpoints:

```
GET https://api.percolist.com.br/updates/windows/x86_64/latest.json
GET https://api.percolist.com.br/updates/linux/x86_64/latest.json
GET https://api.percolist.com.br/updates/darwin/x86_64/latest.json
GET https://api.percolist.com.br/updates/darwin/aarch64/latest.json
```

### 2. Estrutura da Resposta

Cada endpoint deve retornar JSON como este:

```json
{
  "version": "v1.0.2",
  "notes": "Correções de bugs e melhorias",
  "pub_date": "2024-01-15T10:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "Content of Percolist_Panel_1.0.2_x64-setup.exe.sig",
      "url": "https://api.percolist.com.br/downloads/Percolist_Panel_1.0.2_x64-setup.exe"
    }
  }
}
```

### 3. Exemplos Prontos

Execute para gerar exemplos:
```bash
./generate-api-examples.sh
```

Isso criará arquivos em `api-examples/` que você pode usar como base.

## 🔧 Como Fazer Releases

### Opção A: Automatizado (Recomendado)
```bash
./build-for-api.sh 1.0.2 "Correções de bugs e melhorias"
```

### Opção B: Manual
1. Atualize versões nos arquivos
2. `npm run tauri build`
3. Gere assinaturas
4. Faça upload dos binários
5. Configure os endpoints da API

## 📁 Estrutura de Arquivos

### Binários (para upload)
```
src-tauri/target/release/bundle/
├── nsis/Percolist_Panel_1.0.2_x64-setup.exe
├── appimage/Percolist_Panel_1.0.2_amd64.AppImage
├── dmg/Percolist_Panel_1.0.2_x64.dmg
└── dmg/Percolist_Panel_1.0.2_arm64.dmg
```

### Assinaturas (para API)
```
src-tauri/target/release/bundle/
├── nsis/Percolist_Panel_1.0.2_x64-setup.exe.sig
├── appimage/Percolist_Panel_1.0.2_amd64.AppImage.sig
├── dmg/Percolist_Panel_1.0.2_x64.dmg.sig
└── dmg/Percolist_Panel_1.0.2_arm64.dmg.sig
```

### JSON para API (gerado automaticamente)
```
api-files/
├── windows-x86_64.json
├── linux-x86_64.json
├── darwin-x86_64.json
└── darwin-aarch64.json
```

## 🎯 Processo Completo

### 1. Build e Preparação
```bash
./build-for-api.sh 1.0.2 "Nova versão com melhorias"
```

### 2. Upload dos Binários
Faça upload dos arquivos de `src-tauri/target/release/bundle/` para:
```
https://api.percolist.com.br/downloads/
```

### 3. Configurar Endpoints da API
Use os arquivos de `api-files/` para configurar os endpoints da sua API.

### 4. Testar
```bash
npm run tauri dev
```

## 🔍 Troubleshooting

### Erro: "Falha ao verificar atualizações"
- Verifique se os endpoints da API estão funcionando
- Confirme se os JSON estão no formato correto
- Teste os endpoints no navegador

### Erro: "Assinatura inválida"
- Verifique se a chave pública está correta
- Confirme se as assinaturas foram geradas corretamente
- Verifique se a senha da chave privada está correta

### Erro: "Download falhou"
- Verifique se os binários estão acessíveis via URL
- Confirme se as URLs no JSON estão corretas
- Teste o download manualmente

## 🛠️ Comandos Úteis

### Gerar Assinatura Manual
```bash
npm run tauri signer sign arquivo.exe > arquivo.exe.sig
```

### Testar Endpoint
```bash
curl https://api.percolist.com.br/updates/windows/x86_64/latest.json
```

### Verificar Chave Pública
```bash
cat ~/.tauri/percolist.key.pub
```

## 📞 Suporte

Para dúvidas:
1. Verifique os logs do app
2. Teste os endpoints da API
3. Confirme as configurações
4. Consulte a documentação do Tauri v2
