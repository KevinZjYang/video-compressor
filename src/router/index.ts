import { createRouter, createWebHistory } from "vue-router";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: "/compress"
    },
    {
      path: "/compress",
      name: "compress",
      component: () => import("../views/CompressView.vue")
    },
    {
      path: "/trim",
      name: "trim",
      component: () => import("../views/TrimView.vue")
    }
  ]
});

export default router;
