import { createApp } from "vue";
import App from "./App.vue";
import "./assets/main.css";
import { createStore, STORE_KEY } from "./stores/useStore";
import { logError } from "./utils/errorLog";

const app = createApp(App);
app.provide(STORE_KEY, createStore());

app.config.errorHandler = (err, _instance, info) => {
  const msg = err instanceof Error ? `${err.name}: ${err.message}` : String(err);
  logError(`Vue.errorHandler [${info}]`, msg);
};

app.config.warnHandler = (msg, _instance, trace) => {
  logError("Vue.warnHandler", `${msg}\n${trace}`);
};

app.mount("#app");
