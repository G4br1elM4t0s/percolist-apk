#!/bin/bash

WINDOW_TITLE="Percolist_Panel_Window" # Exatamente como no tauri.conf.json
TARGET_WIDTH=1920
TARGET_HEIGHT=32
TARGET_X=0
TARGET_Y=0

# Garante que wmctrl está instalado
if ! command -v wmctrl &> /dev/null; then
    echo "wmctrl não encontrado. Tentando instalar..."
    if sudo apt-get update && sudo apt-get install -y wmctrl; then
        echo "wmctrl instalado com sucesso."
    else
        echo "Falha ao instalar wmctrl. Por favor, instale manualmente." >&2
        exit 1
    fi
fi

echo "Aguardando a janela '$WINDOW_TITLE' aparecer..."

# Espera um pouco para a janela do Tauri ter chance de aparecer
# Tenta encontrar e redimensionar por um curto período
for i in {1..15}; do # Tenta por até 15 segundos (15 * 1s)
    WINDOW_ID=$(wmctrl -l | grep "$WINDOW_TITLE" | awk '{print $1}' | head -n 1)

    if [ -n "$WINDOW_ID" ]; then
        echo "Janela '$WINDOW_TITLE' (ID: $WINDOW_ID) encontrada na tentativa $i."

        # Aplica as configurações agressivamente
        wmctrl -i -r "$WINDOW_ID" -b remove,maximized_vert,maximized_horz,shaded
        wmctrl -i -r "$WINDOW_ID" -b add,above,sticky
        wmctrl -i -r "$WINDOW_ID" -e "0,$TARGET_X,$TARGET_Y,$TARGET_WIDTH,$TARGET_HEIGHT"

        # Pequena pausa e verifica novamente
        sleep 0.2
        NEW_GEOMETRY=$(wmctrl -lG | grep "$WINDOW_ID" | awk '{print $5"x"$6" @ "$3","$4}')
        echo "WMCTRL: Geometria após tentativa de ajuste: $NEW_GEOMETRY"

        # Se o tamanho estiver correto, pode sair ou tentar mais algumas vezes
        # Para garantir, vamos deixar o loop rodar algumas vezes mesmo após sucesso.
        if [[ "$NEW_GEOMETRY" == *"${TARGET_WIDTH}x${TARGET_HEIGHT}"* ]]; then
            echo "WMCTRL: Tamanho correto aparentemente definido."
            if [ $i -gt 5 ]; then # Se já acertou e tentou algumas vezes, pode parar.
                 echo "WMCTRL: Saindo após sucesso e algumas confirmações."
                 exit 0
            fi
        fi
    else
        echo "WMCTRL: Janela '$WINDOW_TITLE' não encontrada (tentativa $i)..."
    fi
    sleep 1 # Espera antes da próxima tentativa
done

echo "WMCTRL: Script finalizado após $i tentativas. Verifique o estado da janela."
exit 0
