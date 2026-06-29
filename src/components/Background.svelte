<!--
  Purely decorative full-screen grid background.

  Note for a new reader: there is no <script> here at all — this component is just
  one <div> plus CSS. `pointer-events: none` means clicks pass straight through it,
  and `z-index: -1` keeps it behind everything else, so it can never get in the way
  of the real UI. A good reminder that not every component needs logic.
-->
<div class="grid-bg"></div>

<style>
  .grid-bg {
    position: fixed;
    inset: 0;
    z-index: -1;
    background-color: #010201;
    background-image: linear-gradient(to right, #0f0f10 1px, transparent 1px),
      linear-gradient(to bottom, #0f0f10 1px, transparent 1px);
    background-size: 1rem 1rem;
    background-position: center center;
    filter: blur(1px);
    pointer-events: none;
  }
</style>