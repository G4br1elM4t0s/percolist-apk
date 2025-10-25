import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';


export const useAuthListener = () => {


  useEffect(() => {
    const unlisten = listen('auth-data-received', (event) => {
      console.log('🎉 Evento auth-data-received recebido:', event);

      try {
        const authData = JSON.parse(event.payload as string);
        console.log('📋 Dados de autenticação:', authData);

        // Salvar no localStorage
        localStorage.setItem('auth_token', authData.token);
        localStorage.setItem('auth_session_id', authData.sessionId);


        console.log('✅ Dados salvos no localStorage e estado atualizado');
      } catch (error) {
        console.error('❌ Erro ao processar dados de autenticação:', error);
      }
    });

    return () => {
      unlisten.then(f => f());
    };
  }, []);
};
