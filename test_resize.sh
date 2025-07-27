#!/bin/bash

echo "=== TESTE DE REDIMENSIONAMENTO FORÇADO ==="

# Lista todas as janelas
echo "Janelas disponíveis:"
wmctrl -l -G

echo ""
echo "Procurando janelas do Percolist/Tauri..."

# Encontra e redimensiona TODAS as janelas suspeitas
wmctrl -l -G | while read line; do
    WINDOW_ID=$(echo "$line" | awk '{print $1}')
    WINDOW_NAME=$(echo "$line" | awk '{for(i=5;i<=NF;i++) printf "%s ", $i; print ""}')

    # Se a janela tem qualquer coisa relacionada ao nosso app
    if [[ $WINDOW_NAME == *"Percolist"* ]] || [[ $WINDOW_NAME == *"percolist"* ]] || [[ $WINDOW_NAME == *"tauri"* ]] || [[ $WINDOW_NAME == *"app"* ]]; then
        echo "REDIMENSIONANDO: $WINDOW_ID - $WINDOW_NAME"

        # Força redimensionamento BRUTAL
        wmctrl -i -r $WINDOW_ID -b remove,maximized_vert,maximized_horz
        wmctrl -i -r $WINDOW_ID -e 0,0,0,1920,32
        wmctrl -i -r $WINDOW_ID -b add,above,sticky

        echo "Redimensionado para 1920x32 na posição 0,0"
    fi
done

echo ""
echo "=== RESULTADO ==="
wmctrl -l -G
