import { useEffect } from 'react';
import ReactDOM from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';

const Splash = () => {
    useEffect(() => {
        // Simular carga o invocar función real
        setTimeout(() => {
            invoke('splash_done');
        }, 5000);
    }, []);

    return <h1>Iniciando Modpack Store...</h1>;
};

ReactDOM.createRoot(document.getElementById('root')!).render(<Splash />);
