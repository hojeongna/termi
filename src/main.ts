import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';

const app = mount(App, {
  // TYPE-ASSERT: index.html guarantees <div id="app"> exists at page load
  target: document.getElementById('app')!,
});

export default app;
