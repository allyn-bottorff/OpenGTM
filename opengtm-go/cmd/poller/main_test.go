package main

import (
	"net"
	"testing"
)

func TestTheTests(t *testing.T) {
	// t.Fatal("failed successfully")
}

func TestDumpTable(t *testing.T) {
	var testTable = new(HealthTable)
	testTable.Health = make(map[string][]Member)
	testTable.Health["pool1"] = []Member{
		{Host: "host1", Ip: net.ParseIP("1.1.1.1"), Healthy: false, Cancel: false, Failures: 0},
		{Host: "host2", Ip: net.ParseIP("1.1.1.1"), Healthy: false, Cancel: false, Failures: 0},
	}
	testTable.Health["pool2"] = []Member{
		{Host: "host3", Ip: net.ParseIP("2.2.2.2"), Healthy: false, Cancel: false, Failures: 0},
		{Host: "host4", Ip: net.ParseIP("2.2.2.2"), Healthy: false, Cancel: false, Failures: 0},
	}
	var tableString, err = testTable.dumpTable()
	if err != nil {
		t.Fatal(err)
	}

	var testString = `{"pool1":[{"host":"host1","ip":"1.1.1.1","healthy":false,"cancel":false,"failures":0},{"host":"host2","ip":"1.1.1.1","healthy":false,"cancel":false,"failures":0}],"pool2":[{"host":"host3","ip":"2.2.2.2","healthy":false,"cancel":false,"failures":0},{"host":"host4","ip":"2.2.2.2","healthy":false,"cancel":false,"failures":0}]}`

	if testString != tableString {
		t.Fatalf("Got: %s  Expected: %s", tableString, testString)
	}

}
