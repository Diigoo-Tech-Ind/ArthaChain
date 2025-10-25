// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title PriceOracle
/// @notice Simple on-chain price oracle for ARTH/GB-month with DAO-controlled floors/ceilings
contract PriceOracle {
    address public dao;
    uint256 public baseWeiPerGBMonth;
    uint256 public floorWeiPerGBMonth;
    uint256 public ceilingWeiPerGBMonth;

    event Updated(uint256 base, uint256 floor, uint256 ceiling);
    event DaoTransferred(address indexed previousDao, address indexed newDao);

    modifier onlyDao() {
        require(msg.sender == dao, "forbidden");
        _;
    }

    constructor(uint256 baseWei, uint256 floorWei, uint256 ceilingWei) {
        dao = msg.sender;
        baseWeiPerGBMonth = baseWei;
        floorWeiPerGBMonth = floorWei;
        ceilingWeiPerGBMonth = ceilingWei;
        emit Updated(baseWei, floorWei, ceilingWei);
    }

    function setDao(address newDao) external onlyDao {
        require(newDao != address(0), "zero");
        address prev = dao;
        dao = newDao;
        emit DaoTransferred(prev, newDao);
    }

    function setPrices(uint256 baseWei, uint256 floorWei, uint256 ceilingWei) external onlyDao {
        require(floorWei <= baseWei && baseWei <= ceilingWei, "bounds");
        baseWeiPerGBMonth = baseWei;
        floorWeiPerGBMonth = floorWei;
        ceilingWeiPerGBMonth = ceilingWei;
        emit Updated(baseWei, floorWei, ceilingWei);
    }

    function getPrice() external view returns (uint256 baseWei, uint256 floorWei, uint256 ceilingWei) {
        return (baseWeiPerGBMonth, floorWeiPerGBMonth, ceilingWeiPerGBMonth);
    }
}


