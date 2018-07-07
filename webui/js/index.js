(function($) {
    var uri = "ws://192.168.0.1:9001";
    var constellation = {
        "GPS": "🇺🇸",
        "SBAS": "SBAS ",
        "Galileo": "🇷🇺",
        "GLONASS": "🇪🇺",
        "Unknown": "Unknown "
    }

    var fix_quality = {
        "TwoDim": "Two dimensional",
        "ThreeDim": "Three dimensional",
        "SBAS": "SBAS (WAAS/EGNOS)",
        "Unknown": "Unknown"
    }

    var nacp = {
        11: "3m",
        10: "10m",
        9: "30m",
        8: "92.6m",
        7: "185.2m",
        6: "555.6m",
        0: ""
    }

    var ws = new WebSocket(uri);
    ws.onopen = function(evt) { $('#conn_stat').text('Connected'); };
    ws.onclose = function(evt) { $('#conn_stat').text('Disconnected'); };
    ws.onmessage = function(evt) {
        var m = JSON.parse(evt.data);

        switch (m.type) {
            case "GNSS":
                $("#num_sv").text(m.num_sv);
                $("#fix_quality").text(fix_quality[m.quality]);

                var html = "";
                for (i = 0; i < m.sv_status.length; i++) {
                    var s = m.sv_status[i];

                    html += "<tr>" +
                                "<td>" + constellation[s.system] + s.sv_id + "</td>" +
                                "<td>" + (s.acquired ? "Yes" : "No") + "</td>" +
                                "<td>" + (s.in_solution ? "Yes" : "No") + "</td>" +
                                "<td>" + s.signal + "</td>" +
                                "<td>" + s.azimuth + "</td>" +
                                "<td>" + s.elevation + "</td>" +
                            "</tr>"
                }

                $('#sv_status > tbody').empty().append(html);
                break;

            case "Ownship":
                $('#lat').text(m.lat.toFixed(4));
                $('#lon').text(m.lon.toFixed(4));
                $('#alt').text(m.altitude);
                $('#track').text(m.track.toFixed(0));
                $('#nacp').text('(' + nacp[m.nacp] + ')');
                $('#gs').text(m.gs.toFixed(0));
                break;
        }
    };
    ws.onerror = function(evt) { console.log(evt) };


})(jQuery);
