
import ReactDOM from 'react-dom/client';
import App from './App';
import { ErrorBoundary } from './components/ErrorBoundary';
import './index.css';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  // BEZ <React.StrictMode>
  <ErrorBoundary>
    <App />
  </ErrorBoundary>
)