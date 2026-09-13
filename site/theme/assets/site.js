// Mobile menu. No tracking, no third-party requests.
const toggle = document.querySelector(".menu-toggle");
const nav = document.getElementById("navigation");
if (toggle && nav) {
  toggle.addEventListener("click", () => {
    const open = nav.classList.toggle("open");
    toggle.setAttribute("aria-expanded", String(open));
  });
}
