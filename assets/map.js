var map;
var markers = new Map();
var focusedMarker;
var destinations = [];

var apartIcon = L.divIcon({className: 'apartment-icon'});
var apartHoverIcon = L.divIcon({className: 'apartment-hover-icon'});

function initMap() {
  map = L.map('map');
  var layer = L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Topo_Map/MapServer/tile/{z}/{y}/{x}', {
    attribution: 'Tiles &copy; Esri'
  });
  layer.addTo(map);
}

function addDest(lng, lat, color) {
  var destIcon = L.divIcon({className: "none", iconSize: [20, 20], html: `<div style="width: 100%; height: 100%; background: ${color}99; border: 2px solid ${color}; border-radius: 50%"></div>`});
  var destination = L.marker(L.latLng(lat, lng), {icon: destIcon});
  destination.addTo(map);
  destinations.push(destination);
}

function clickMarker(e) {
  var name = e.target.getTooltip().getElement().innerText.trim();
  var names = document.querySelectorAll(".building-head h3")
  var name = Array.from(names).find(elt => elt.textContent.trim() === name);
  name.scrollIntoView();
}

function addMarker(name, lng, lat) {
  var point = L.latLng(lat, lng);
  var marker = L.marker(point, {icon: apartIcon}).addTo(map);
  marker.bindTooltip(name);
  marker.on('click', clickMarker);
  markers.set(name, marker);
}

function clearMap() {
  destinations.forEach((marker) => marker.remove());
  markers.forEach((marker, _) => marker.remove());

  destinations = [];
  markers.clear();
  focusedMarker = null;
}

function fitMap() {
  var bounds = [...markers.values(), ...destinations].map((marker) => marker.getLatLng());
  map.invalidateSize();
  map.fitBounds(bounds);
}

function focusMarker(name) {
  focusedMarker = markers.get(name);
  focusedMarker.setIcon(apartHoverIcon);
  focusedMarker.setZIndexOffset(10000);
  var bounds = [focusedMarker, ...destinations].map((marker) => marker.getLatLng());
  map.fitBounds(bounds);
}

function unfocusMarker() {
  focusedMarker.setIcon(apartIcon);
  focusedMarker.setZIndexOffset(0);
  fitMap();
}
