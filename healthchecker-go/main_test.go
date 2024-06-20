package main

import (
	"testing"
)

func TestHttpReceiveValidate(t *testing.T) {
	hr := HttpReceive{
		StatusCodes: &[]uint{200, 201},
		String:      nil,
	}
	err := hr.validate()
	if err != nil {
		t.Fatalf("%v", err)
	}

	receiveString := "hello"
	hr = HttpReceive{
		StatusCodes: nil,
		String:      &receiveString,
	}
	err = hr.validate()
	if err != nil {
		t.Fatalf("%v", err)
	}

	hr = HttpReceive{
		StatusCodes: nil,
		String:      nil,
	}
	err = hr.validate()
	if err == nil {
		t.Fatalf("%v", err)
	}

}
