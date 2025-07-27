#!/bin/bash

FORCE_SIZE_SCRIPT="./force_window_size.sh"

# Tenta carregar NVM se existir, para garantir que node/npm estejam no PATH
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"  # Carrega nvm
[ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"  # Carrega nvm bash_completion

# Verifica se npm está disponível
if ! command -v npm &> /dev/null; then
    echo "Erro: Comando NPM não encontrado. Certifique-se de que Node.js e NPM estão instalados e no PATH." >&2
    echo "Se você usa NVM, tente executar 'nvm use node' ou similar no seu terminal antes de rodar este script." >&2
    exit 1
fi

TAURI_DEV_CMD="npm run tauri dev"

echo "Iniciando Percolist com correção automática de tamanho..."

# Garante que o script de redimensionamento é executável
chmod +x "$FORCE_SIZE_SCRIPT" || exit 1

# Mata processos anteriores para um início limpo
echo "Finalizando processos anteriores (se existirem)..."
pkill -f "$FORCE_SIZE_SCRIPT" || true
pkill -f "target/debug/app" || true # Processo principal do Tauri
pkill -f "Percolist_Panel_Window" || true # Tenta matar pela string do título

# Inicia o script de redimensionamento em background
echo "Iniciando script de redimensionamento em background..."
"$FORCE_SIZE_SCRIPT" & # Executa em background
RESIZE_PID=$!

echo "Aguardando brevemente para o script de redimensionamento iniciar (e o Tauri compilar, se necessário)..."
sleep 5 # Aumentar um pouco o sleep para dar tempo ao Tauri compilar na primeira vez

# Inicia o Percolist (Tauri)
echo "Iniciando Percolist (Tauri)..."
$TAURI_DEV_CMD &
TAURI_PID=$!

# Monitora o processo do Tauri e finaliza o script de resize quando o Tauri fechar
wait $TAURI_PID
EXIT_CODE=$?

echo "Aplicação Tauri finalizada com código: $EXIT_CODE"
echo "Finalizando script de redimensionamento (PID: $RESIZE_PID)..."
kill $RESIZE_PID 2>/dev/null || true
pkill -f "$FORCE_SIZE_SCRIPT" || true # Garante que o script wmctrl morra

echo "Percolist e script de redimensionamento finalizados."
exit $EXIT_CODE