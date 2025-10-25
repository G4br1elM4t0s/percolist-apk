#!/bin/bash

# Script para build e release do Percolist
# Uso: ./build-and-release.sh <version> <release_notes>

if [ $# -lt 2 ]; then
    echo "Uso: $0 <version> <release_notes>"
    echo "Exemplo: $0 1.0.2 'Correções de bugs e melhorias'"
    exit 1
fi

VERSION=$1
RELEASE_NOTES=$2

echo "🚀 Iniciando build e release para versão $VERSION"

# 1. Atualizar versões nos arquivos
echo "📝 Atualizando versões..."
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" src-tauri/Cargo.toml
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" package.json

# 2. Build do projeto
echo "🔨 Fazendo build..."
export TAURI_SIGNING_PRIVATE_KEY="~/.tauri/percolist.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="sua_senha_aqui"

npm run tauri build

# 3. Gerar assinaturas
echo "🔐 Gerando assinaturas..."
cd src-tauri/target/release/bundle

# Para Windows
if [ -f "nsis/Percolist_Panel_${VERSION}_x64-setup.exe" ]; then
    tauri signer sign "nsis/Percolist_Panel_${VERSION}_x64-setup.exe" > "nsis/Percolist_Panel_${VERSION}_x64-setup.exe.sig"
fi

# Para Linux
if [ -f "appimage/Percolist_Panel_${VERSION}_amd64.AppImage" ]; then
    tauri signer sign "appimage/Percolist_Panel_${VERSION}_amd64.AppImage" > "appimage/Percolist_Panel_${VERSION}_amd64.AppImage.sig"
fi

# Para macOS
if [ -f "dmg/Percolist_Panel_${VERSION}_x64.dmg" ]; then
    tauri signer sign "dmg/Percolist_Panel_${VERSION}_x64.dmg" > "dmg/Percolist_Panel_${VERSION}_x64.dmg.sig"
fi

if [ -f "dmg/Percolist_Panel_${VERSION}_arm64.dmg" ]; then
    tauri signer sign "dmg/Percolist_Panel_${VERSION}_arm64.dmg" > "dmg/Percolist_Panel_${VERSION}_arm64.dmg.sig"
fi

cd ../../..

# 4. Atualizar latest.json
echo "📄 Atualizando latest.json..."
cat > latest.json << EOF
{
  "version": "v$VERSION",
  "notes": "$RELEASE_NOTES",
  "pub_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "platforms": {
    "darwin-x86_64": {
      "signature": "$(cat src-tauri/target/release/bundle/dmg/Percolist_Panel_${VERSION}_x64.dmg.sig 2>/dev/null || echo 'Content of signature file')",
      "url": "https://github.com/seu-usuario/seu-repo/releases/download/v$VERSION/Percolist_Panel_${VERSION}_x64.dmg"
    },
    "darwin-aarch64": {
      "signature": "$(cat src-tauri/target/release/bundle/dmg/Percolist_Panel_${VERSION}_arm64.dmg.sig 2>/dev/null || echo 'Content of signature file')",
      "url": "https://github.com/seu-usuario/seu-repo/releases/download/v$VERSION/Percolist_Panel_${VERSION}_arm64.dmg"
    },
    "linux-x86_64": {
      "signature": "$(cat src-tauri/target/release/bundle/appimage/Percolist_Panel_${VERSION}_amd64.AppImage.sig 2>/dev/null || echo 'Content of signature file')",
      "url": "https://github.com/seu-usuario/seu-repo/releases/download/v$VERSION/Percolist_Panel_${VERSION}_amd64.AppImage"
    },
    "windows-x86_64": {
      "signature": "$(cat src-tauri/target/release/bundle/nsis/Percolist_Panel_${VERSION}_x64-setup.exe.sig 2>/dev/null || echo 'Content of signature file')",
      "url": "https://github.com/seu-usuario/seu-repo/releases/download/v$VERSION/Percolist_Panel_${VERSION}_x64-setup.exe"
    }
  }
}
EOF

echo "✅ Build e release concluído!"
echo "📋 Próximos passos:"
echo "1. Faça commit das mudanças: git add . && git commit -m 'Release v$VERSION'"
echo "2. Crie uma tag: git tag v$VERSION"
echo "3. Push: git push && git push --tags"
echo "4. Crie um release no GitHub com os arquivos de src-tauri/target/release/bundle/"
echo "5. Atualize o latest.json no seu repositório ou GitHub Gist"
