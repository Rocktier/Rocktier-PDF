import { createRoot } from 'react-dom/client';
import { App } from './App';
import { LanguageProvider } from './i18n';

import './styles/tokens.css';
import './styles/global.css';
import './styles/app.css';

const container = document.getElementById('root');
if (!container) throw new Error('#root not found');

// StrictMode is intentionally omitted: its double-invoked effects would fire
// every page render twice in dev, which is misleading when profiling.
createRoot(container).render(
  <LanguageProvider>
    <App />
  </LanguageProvider>
);
