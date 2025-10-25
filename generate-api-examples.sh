#!/bin/bash

echo "🔧 Gerando exemplos para sua API"
echo "================================="

# Criar diretório para exemplos
mkdir -p api-examples

# Windows
cat > api-examples/windows-x86_64.json << 'EOF'
{
  "version": "v1.0.2",
  "notes": "Correções de bugs e melhorias de performance",
  "pub_date": "2024-01-15T10:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "Content of Percolist_Panel_1.0.2_x64-setup.exe.sig",
      "url": "https://api.percolist.com.br/downloads/Percolist_Panel_1.0.2_x64-setup.exe"
    }
  }
}
EOF

# Linux
cat > api-examples/linux-x86_64.json << 'EOF'
{
  "version": "v1.0.2",
  "notes": "Correções de bugs e melhorias de performance",
  "pub_date": "2024-01-15T10:00:00Z",
  "platforms": {
    "linux-x86_64": {
      "signature": "Content of Percolist_Panel_1.0.2_amd64.AppImage.sig",
      "url": "https://api.percolist.com.br/downloads/Percolist_Panel_1.0.2_amd64.AppImage"
    }
  }
}
EOF

# macOS Intel
cat > api-examples/darwin-x86_64.json << 'EOF'
{
  "version": "v1.0.2",
  "notes": "Correções de bugs e melhorias de performance",
  "pub_date": "2024-01-15T10:00:00Z",
  "platforms": {
    "darwin-x86_64": {
      "signature": "Content of Percolist_Panel_1.0.2_x64.dmg.sig",
      "url": "https://api.percolist.com.br/downloads/Percolist_Panel_1.0.2_x64.dmg"
    }
  }
}
EOF

# macOS Apple Silicon
cat > api-examples/darwin-aarch64.json << 'EOF'
{
  "version": "v1.0.2",
  "notes": "Correções de bugs e melhorias de performance",
  "pub_date": "2024-01-15T10:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "Content of Percolist_Panel_1.0.2_arm64.dmg.sig",
      "url": "https://api.percolist.com.br/downloads/Percolist_Panel_1.0.2_arm64.dmg"
    }
  }
}
EOF

echo "✅ Exemplos gerados em api-examples/"
echo ""
echo "📋 Endpoints que sua API precisa implementar:"
echo ""
echo "Windows:"
echo "GET https://api.percolist.com.br/updates/windows/x86_64/latest.json"
echo "Resposta: api-examples/windows-x86_64.json"
echo ""
echo "Linux:"
echo "GET https://api.percolist.com.br/updates/linux/x86_64/latest.json"
echo "Resposta: api-examples/linux-x86_64.json"
echo ""
echo "macOS Intel:"
echo "GET https://api.percolist.com.br/updates/darwin/x86_64/latest.json"
echo "Resposta: api-examples/darwin-x86_64.json"
echo ""
echo "macOS Apple Silicon:"
echo "GET https://api.percolist.com.br/updates/darwin/aarch64/latest.json"
echo "Resposta: api-examples/darwin-aarch64.json"
echo ""
echo "🔧 Para gerar assinaturas reais:"
echo "npm run tauri signer sign arquivo.exe > arquivo.exe.sig"
