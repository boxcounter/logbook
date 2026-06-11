import { createApp } from "vue";
import App from "./App.vue";
import "./assets/main.css";
import { createStore, provideStore } from "./stores/useStore";

const app = createApp(App);
const store = createStore();
provideStore(store);
app.mount("#app");
