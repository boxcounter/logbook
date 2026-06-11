import { createApp } from "vue";
import App from "./App.vue";
import "./assets/main.css";
import { createStore, STORE_KEY } from "./stores/useStore";

const app = createApp(App);
app.provide(STORE_KEY, createStore());
app.mount("#app");
