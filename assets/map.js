var map;
var markers = new Map();
var focusedMarker;
var destination;

var destIcon = L.divIcon({className: 'dest-icon'});
var apartIcon = L.divIcon({className: 'apartment-icon'});
var apartHoverIcon = L.divIcon({className: 'apartment-hover-icon'// , iconSize: [100, 100]
                               });

function initMap() {
  map = L.map('map');
  var layer = L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Topo_Map/MapServer/tile/{z}/{y}/{x}', {
    attribution: 'Tiles &copy; Esri'
  });
  layer.addTo(map);
}

function addDest(lng, lat) {
  destination = L.marker(L.latLng(lat, lng), {icon: destIcon});
  destination.addTo(map);
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
  if (destination != null) { destination.remove() };
  markers.forEach((marker, _) => marker.remove());

  destination = null;
  markers.clear();
  focusedMarker = null;
}

function fitMap() {
  var bounds = [...markers.values(), destination].map((marker) => marker.getLatLng());
  // map.panTo(destination);
  map.invalidateSize();
  map.fitBounds(bounds);
}

function focusMarker(name) {
  focusedMarker = markers.get(name);
  focusedMarker.setIcon(apartHoverIcon);
  focusedMarker.setZIndexOffset(10000);
  map.fitBounds([destination.getLatLng(), focusedMarker.getLatLng()]);
}

function unfocusMarker() {
  focusedMarker.setIcon(apartIcon);
  focusedMarker.setZIndexOffset(0);
  fitMap();
}
