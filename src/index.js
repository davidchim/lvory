import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import './assets/css/global.css';
import setupTauriAdapter from './tauri-adapter';

// 初始化 Tauri 适配器 (仅在 Tauri 环境下生效)
setupTauriAdapter();

const root = createRoot(document.getElementById('root'));
root.render(<App />); 