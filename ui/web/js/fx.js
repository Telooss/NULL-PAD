// Mode « effets coupes » : classe no-fx sur .nc-app (?nofx=1 ou case de Reglages, gardee dans localStorage).
const KEY = 'nullctl.nofx';
const read = () => { try { return localStorage.getItem(KEY) === '1'; } catch { return false; } };

export function fxOff() { return document.querySelector('.nc-app')?.classList.contains('no-fx') ?? false; }
export function setFxOff(on) {
  document.querySelector('.nc-app')?.classList.toggle('no-fx', on);
  try { localStorage.setItem(KEY, on ? '1' : '0'); } catch { /* stockage indisponible : valable pour cette session */ }
}
export function initFx() {
  const url = new URLSearchParams(location.search).get('nofx');
  document.querySelector('.nc-app')?.classList.toggle('no-fx', url === '1' || (url === null && read()));
}
