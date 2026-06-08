import React from 'react';
import ReactDOM from 'react-dom/client';
import { PrivyProvider } from '@privy-io/react-auth';
import App from './app';

// Make React global for JSX files that reference it
window.React = React;
window.ReactDOM = ReactDOM;

// Import utility modules that initialize globals
import './utils/data';
import './components/primitives';

// Import styles
import '../src/styles/styles.css';
import '../src/pages/landing.css';
import '../src/styles/screens.css';
import '../src/styles/dashboard.css';
import '../src/pages/agent.css';
import '../src/pages/identity.css';
import '../src/pages/arena.css';

ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode>
    <PrivyProvider appId={import.meta.env.VITE_PRIVY_APP_ID || ''} config={{
      loginMethods: ['email', 'google', 'github', 'twitter', 'discord', 'wallet'],
      embeddedWallets: {
        createOnLogin: 'users-without-wallets',
      },
      appearance: {
        theme: 'dark',
        accentColor: '#C8F230',
        logo: '',
      },
    }}>
      <App />
    </PrivyProvider>
  </React.StrictMode>,
);
