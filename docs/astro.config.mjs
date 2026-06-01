// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
export default defineConfig({
  site: "https://zooeywm.github.io",
  base: "/Germinal/",
  integrations: [
    starlight({
      title: "Germinal",
      social: [{ icon: "github", label: "GitHub", href: "https://github.com/zooeywm/Germinal" }],
      description: "Documentation for Germinal, the Graphical Terminal.",
      sidebar: [
        {
          label: "Design Documents",
          collapsed: false,
          items: [{ autogenerate: { directory: "design" } }],
        },
      ],
    }),
  ],
});
